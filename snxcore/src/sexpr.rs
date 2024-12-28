use std::{collections::BTreeMap, fmt, str::FromStr};

use anyhow::anyhow;
use num_traits::Num;
use pest::{iterators::Pairs, Parser};
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;

type RulePairs<'a> = Pairs<'a, Rule>;

#[derive(Parser)]
#[grammar = "sexpr.pest"]
struct SExpressionParser;

#[derive(Debug, Clone, PartialEq, enum_as_inner::EnumAsInner)]
pub enum SExpression {
    Null,
    Value(String),
    Object(Option<String>, BTreeMap<String, SExpression>),
    Array(Vec<SExpression>),
}

impl SExpression {
    pub fn object_name(&self) -> Option<&str> {
        self.as_object().and_then(|(n, _)| n.as_deref())
    }

    pub fn try_into<D>(self) -> anyhow::Result<D>
    where
        for<'a> D: Deserialize<'a>,
    {
        Ok(serde_json::from_value(self.to_json())?)
    }

    pub fn get(&self, path: &str) -> Option<&SExpression> {
        let parts = path.split(':');
        self.get_for_parts(parts)
    }

    pub fn get_value<T: FromStr>(&self, path: &str) -> Option<T> {
        self.get(path).and_then(|v| v.as_value()).and_then(|v| v.parse().ok())
    }

    pub fn get_num_value<T: Num>(&self, path: &str) -> Option<T> {
        self.get(path)
            .and_then(|v| v.as_value())
            .and_then(|v| parse_int::parse(v).ok())
    }

    fn get_for_parts<'a, I>(&self, parts: I) -> Option<&SExpression>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut iter = parts.into_iter();

        match iter.next() {
            Some("") => self.get_for_parts(iter),
            Some(name) => match self {
                SExpression::Object(maybe_name, fields)
                    if maybe_name.as_ref().is_some_and(|n| n == name) || maybe_name.is_none() =>
                {
                    let name = if maybe_name.is_none() { Some(name) } else { iter.next() };
                    match name {
                        Some(name) => {
                            for (f_name, f_value) in fields {
                                if f_name == name {
                                    return f_value.get_for_parts(iter);
                                }
                            }
                            None
                        }
                        None => None,
                    }
                }
                SExpression::Array(items) => match name.parse::<usize>() {
                    Ok(index) => items.get(index).and_then(|item| item.get_for_parts(iter)),
                    Err(_) => None,
                },
                _ => None,
            },
            _ => Some(self),
        }
    }

    fn encode_with_level(&self, level: u32) -> Option<String> {
        match self {
            SExpression::Null => None,
            SExpression::Value(value) => Some(format_value(value)),
            SExpression::Object(name, object) => Some(self.encode_object(level, name.as_deref(), object)),
            SExpression::Array(items) => Some(self.encode_array(level, items)),
        }
    }

    fn encode_object<K: AsRef<str>>(
        &self,
        level: u32,
        name: Option<&str>,
        object: &BTreeMap<K, SExpression>,
    ) -> String {
        let fields = object
            .iter()
            .filter_map(|(k, v)| {
                let v = v.encode_with_level(level + 1);
                v.map(|v| format!("{}:{} {}", indent(level + 1), k.as_ref(), v))
            })
            .collect::<Vec<String>>()
            .join("\n");
        format!("({}\n{})", name.unwrap_or(""), fields)
    }

    fn encode_array(&self, level: u32, items: &[SExpression]) -> String {
        let formatted_items = items
            .iter()
            .filter_map(|item| {
                let v = item.encode_with_level(level + 1);
                v.map(|v| format!("{}: {}", indent(level + 1), v))
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("(\n{formatted_items})")
    }

    pub fn to_json(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Value(v) => to_json_value(v),
            Self::Object(name, fields) => to_json_object(name.as_deref(), fields),
            SExpression::Array(elements) => Value::Array(elements.iter().map(|v| v.to_json()).collect()),
        }
    }

    fn from_json(json: Value) -> Self {
        match json {
            Value::Null => Self::Null,
            Value::Bool(v) => Self::Value(v.to_string()),
            Value::Number(v) => Self::Value(v.to_string()),
            Value::String(v) => Self::Value(v.to_string()),
            Value::Array(v) => Self::Array(v.into_iter().map(Self::from_json).collect()),
            Value::Object(v) => match v.iter().next() {
                Some((key, value)) if key.starts_with('(') => Self::Object(
                    Some(key[1..].to_string()),
                    value
                        .as_object()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), Self::from_json(v.clone())))
                        .collect(),
                ),
                _ => Self::Object(None, v.into_iter().map(|(k, v)| (k, Self::from_json(v))).collect()),
            },
        }
    }
}

impl<T: Serialize> From<T> for SExpression {
    fn from(value: T) -> Self {
        let json = serde_json::to_value(value).unwrap_or_default();
        Self::from_json(json)
    }
}

impl<T> TryFrom<SExpression> for (T,)
where
    for<'a> T: Deserialize<'a>,
{
    type Error = anyhow::Error;

    fn try_from(value: SExpression) -> Result<Self, Self::Error> {
        Ok(serde_json::from_value(value.to_json())?)
    }
}

impl FromStr for SExpression {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rules = SExpressionParser::parse(Rule::top, s)?;
        parse_sexpr(rules)
    }
}

impl fmt::Display for SExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode_with_level(0).unwrap_or_default())
    }
}

fn to_json_value(v: &str) -> Value {
    if let Ok(i) = v.parse::<u32>() {
        Value::Number(i.into())
    } else if let Ok(h) = u32::from_str_radix(v.trim_start_matches("0x"), 16) {
        Value::Number(h.into())
    } else if let Ok(b) = v.parse::<bool>() {
        Value::Bool(b)
    } else {
        Value::String(v.to_string())
    }
}

fn format_value(value: &str) -> String {
    if value.contains(|c: char| !c.is_alphanumeric()) {
        format!("(\"{}\")", value)
    } else {
        format!("({})", value)
    }
}

fn to_json_object<N: AsRef<str>, K: AsRef<str>>(name: Option<N>, fields: &BTreeMap<K, SExpression>) -> Value {
    let inner = Value::Object(
        fields
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.to_json()))
            .collect(),
    );
    if let Some(name) = name {
        Value::Object([(format!("({}", name.as_ref()), inner)].into_iter().collect())
    } else {
        inner
    }
}

fn indent(level: u32) -> String {
    (0..level).map(|_| "\t").collect()
}

fn parse_sexpr(mut pairs: RulePairs) -> anyhow::Result<SExpression> {
    match pairs.next() {
        None => Ok(SExpression::Value(String::new())),
        Some(rule) if rule.as_rule() == Rule::obj => parse_obj(rule.into_inner()),
        Some(rule) if rule.as_rule() == Rule::array => parse_array(rule.into_inner()),
        Some(rule) if rule.as_rule() == Rule::value => parse_value(rule.into_inner()),
        other => Err(anyhow!("Invalid sexpr: {:?}", other)),
    }
}

fn parse_obj(pairs: RulePairs) -> anyhow::Result<SExpression> {
    let mut name: Option<String> = None;
    let mut fields = BTreeMap::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::ident => {
                name = Some(pair.as_str().to_owned());
            }
            Rule::field => {
                let (key, value) = parse_field(pair.into_inner())?;
                fields.insert(key, value);
            }
            _ => return Err(anyhow!("Invalid object")),
        }
    }
    Ok(SExpression::Object(name, fields))
}

fn parse_field(mut pairs: RulePairs) -> anyhow::Result<(String, SExpression)> {
    let rule = pairs.next().ok_or_else(|| anyhow!("Invalid field"))?;
    let name = rule.as_str()[1..].to_owned();
    let value = parse_sexpr(pairs)?;
    Ok((name, value))
}

fn parse_array(pairs: RulePairs) -> anyhow::Result<SExpression> {
    let mut array = Vec::new();
    for pair in pairs {
        array.push(parse_sexpr(pair.into_inner())?);
    }
    Ok(SExpression::Array(array))
}

fn parse_value(mut pairs: RulePairs) -> anyhow::Result<SExpression> {
    match pairs.next() {
        Some(pair) if pair.as_rule() == Rule::quoted_str => {
            let value = pair.into_inner().as_str().to_owned();
            Ok(SExpression::Value(value))
        }
        Some(pair) if pair.as_rule() == Rule::simple_val => {
            let value = pair.as_str().to_owned();
            Ok(SExpression::Value(value))
        }
        _ => Err(anyhow!("Invalid value")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_client_hello() {
        let data = std::fs::read_to_string("tests/client_hello.txt").unwrap();
        let expr = data.parse::<SExpression>().unwrap();

        println!("{expr:#?}");
        println!("{expr}");
    }

    #[test]
    fn test_parse_hello_reply() {
        let data = std::fs::read_to_string("tests/hello_reply.txt").unwrap();
        let expr = data.parse::<SExpression>().unwrap();

        println!("{expr:#?}");
        println!("{expr}");

        assert_eq!(
            expr.get("hello_reply:range:0:from"),
            Some(&SExpression::Value("10.0.0.0".to_string()))
        );

        assert_eq!(
            expr.get("hello_reply:range:0:to"),
            Some(&SExpression::Value("10.255.255.255".to_string()))
        );

        assert_eq!(
            expr.get("hello_reply:range:1"),
            Some(&SExpression::Object(
                None,
                BTreeMap::from([
                    ("from".to_owned(), SExpression::Value("172.16.0.0".to_string())),
                    ("to".to_owned(), SExpression::Value("172.16.255.255".to_string()))
                ])
            ))
        );

        let json = expr.to_json();
        println!("{json:#?}");

        let from_json = SExpression::from_json(json);
        assert_eq!(from_json, expr);
    }

    #[test]
    fn test_parse_client_request() {
        let data = std::fs::read_to_string("tests/client_request.txt").unwrap();
        let expr = data.parse::<SExpression>().unwrap();
        println!("{expr:#?}");
        println!("{expr}");
    }

    #[test]
    fn test_parse_server_response() {
        let data = std::fs::read_to_string("tests/server_response.txt").unwrap();
        let expr = data.parse::<SExpression>().unwrap();
        println!("{expr:#?}");
        println!("{expr}");
    }

    #[test]
    fn test_parse_array() {
        let data = "(Response :data (: (hello) : (world)))";
        let expr = data.parse::<SExpression>().unwrap();
        assert_eq!(
            expr.get("Response:data").unwrap().as_array().unwrap(),
            &vec![
                SExpression::Value("hello".to_string()),
                SExpression::Value("world".to_string())
            ]
        );
    }

    #[test]
    fn test_empty() {
        #[derive(Serialize)]
        struct Data {
            key: Option<u32>,
        }
        let data = Data { key: None };
        let expr = SExpression::from(&data);
        assert_eq!(expr.get("key"), Some(&SExpression::Null));
    }

    #[test]
    fn test_quoted_value_from_str() {
        let data = "(Response\n\t:data (\"hello world\"))";
        let expr = data.parse::<SExpression>().unwrap();

        let inner = expr.get("Response:data").unwrap().as_value().unwrap();
        assert_eq!(inner, "hello world");

        let encoded = format!("{}", expr);
        assert_eq!(encoded, data);
    }

    #[test]
    fn test_quoted_value_from_pod() {
        #[derive(Serialize)]
        struct Data {
            key: String,
        }
        let data = Data {
            key: "Helloworld!".to_owned(),
        };
        let expr = SExpression::from(&data);

        let inner = expr.get("key").unwrap().as_value().unwrap();
        assert_eq!(inner, "Helloworld!");

        let encoded = format!("{}", expr);
        assert_eq!(encoded, "(\n\t:key (\"Helloworld!\"))");
    }
}
