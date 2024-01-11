use anyhow::anyhow;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;

type RulePair<'a> = Pair<'a, Rule>;
type RulePairs<'a> = Pairs<'a, Rule>;

#[derive(Parser)]
#[grammar = "sexpr.pest"]
struct SExpression;

pub fn decode<S, T>(expression: S) -> anyhow::Result<(String, T)>
where
    S: AsRef<str>,
    for<'a> T: Deserialize<'a>,
{
    let mut rules = SExpression::parse(Rule::sexpr, expression.as_ref())?;

    let command = parse_command(rules.next().ok_or_else(|| anyhow!("No command"))?)?;
    let data = parse_data(rules.next().ok_or_else(|| anyhow!("No data"))?)?;

    Ok((command, serde_json::from_value(data)?))
}

pub fn encode<S, T>(name: S, data: T) -> anyhow::Result<String>
where
    S: AsRef<str>,
    T: Serialize,
{
    let json = serde_json::to_value(data)?;
    value_to_s_expr(json, 1)
        .map(|v| format!("({}{})", name.as_ref(), v))
        .ok_or_else(|| anyhow!("No value to format!"))
}

pub fn encode_value<T>(data: T) -> anyhow::Result<String>
where
    T: Serialize,
{
    let json = serde_json::to_value(data)?;
    value_to_s_expr(json, 1).ok_or_else(|| anyhow!("No value to format!"))
}

fn indent(level: u32) -> String {
    (0..level).map(|_| "\t").collect()
}

fn value_to_s_expr(value: Value, level: u32) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(num) => Some(num.to_string()),
        Value::Array(array) => Some(
            array
                .into_iter()
                .filter_map(|v| value_to_s_expr(v, level + 1).map(|v| format!("\n{}: ({})", indent(level), v)))
                .collect::<Vec<_>>()
                .join(""),
        ),
        Value::Object(map) => Some(
            map.into_iter()
                .filter_map(|(k, v)| {
                    value_to_s_expr(v, level + 1).map(|v| format!("\n{}:{} ({})", indent(level), k, v))
                })
                .collect::<Vec<_>>()
                .join(""),
        ),
    }
}

fn parse_command(pair: RulePair) -> anyhow::Result<String> {
    match pair.as_rule() {
        Rule::command => Ok(pair.as_str().to_owned()),
        _ => Err(anyhow!("Not a command")),
    }
}

fn parse_data(pair: RulePair) -> anyhow::Result<Value> {
    match pair.as_rule() {
        Rule::obj => parse_obj(pair.into_inner()),
        Rule::array => parse_array(pair.into_inner()),
        Rule::value => {
            if let Ok(i) = pair.as_str().parse::<u32>() {
                Ok(Value::Number(i.into()))
            } else if let Ok(h) = u32::from_str_radix(pair.as_str().trim_start_matches("0x"), 16) {
                Ok(Value::Number(h.into()))
            } else if let Ok(b) = pair.as_str().parse::<bool>() {
                Ok(Value::Bool(b))
            } else {
                Ok(Value::String(pair.as_str().to_owned()))
            }
        }
        _ => Err(anyhow!("Invalid rule")),
    }
}

fn parse_obj(pairs: RulePairs) -> anyhow::Result<Value> {
    let mut map = serde_json::Map::new();
    for pair in pairs {
        match pair.as_rule() {
            Rule::field => {
                let (key, value) = parse_field(pair.into_inner())?;
                map.insert(key, value);
            }
            _ => return Err(anyhow!("Not a field")),
        }
    }
    Ok(Value::Object(map))
}

fn parse_field(mut pairs: RulePairs) -> anyhow::Result<(String, Value)> {
    let rule = pairs.next().ok_or_else(|| anyhow!("No name"))?;
    let key = match rule.as_rule() {
        Rule::ident => rule.as_str().to_owned(),
        _ => return Err(anyhow!("Not an ident")),
    };
    match pairs.next() {
        Some(rule) => {
            let value = parse_data(rule)?;
            Ok((key, value))
        }
        None => Ok((key, Value::String(String::new()))),
    }
}

fn parse_array(pairs: RulePairs) -> anyhow::Result<Value> {
    let mut array = Vec::new();
    for pair in pairs {
        match pair.as_rule() {
            Rule::elem => {
                let value = parse_data(pair.into_inner().next().ok_or_else(|| anyhow!("No value"))?)?;
                array.push(value);
            }
            _ => return Err(anyhow!("Not an elem")),
        }
    }
    Ok(Value::Array(array))
}

#[cfg(test)]
mod tests {
    use crate::model::proto::{CccClientRequest, CccServerResponse, ClientHello, HelloReply};

    use super::*;

    #[test]
    fn test_parse_client_hello() {
        let data = std::fs::read_to_string("tests/client_hello.txt").unwrap();
        let expr = decode::<_, ClientHello>(data).unwrap();

        println!("{:#?}", expr);
    }

    #[test]
    fn test_parse_hello_reply() {
        let data = std::fs::read_to_string("tests/hello_reply.txt").unwrap();
        let (_, reply) = decode::<_, HelloReply>(data).unwrap();

        println!("{:#?}", reply);

        let s_expr = encode(HelloReply::NAME, &reply).unwrap();
        println!("{}", s_expr);

        let (_, reparsed) = decode::<_, HelloReply>(&s_expr).unwrap();
        assert_eq!(reparsed, reply);
    }

    #[test]
    fn test_parse_client_request() {
        let data = std::fs::read_to_string("tests/client_request.txt").unwrap();
        let expr = decode::<_, CccClientRequest>(data).unwrap();
        println!("{:#?}", expr);
    }

    #[test]
    fn test_parse_server_response() {
        let data = std::fs::read_to_string("tests/server_response.txt").unwrap();
        let expr = decode::<_, CccServerResponse>(data).unwrap();
        println!("{:#?}", expr);
    }

    #[test]
    fn test_parse_array() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Response {
            data: Vec<String>,
        }
        let data = "(Response :data (:1 (hello) :200 (world)))";
        let expr = decode::<_, Response>(data).unwrap();
        assert_eq!(
            expr.1,
            Response {
                data: vec!["hello".to_string(), "world".to_string()],
            },
        );

        println!("{:#?}", expr);
    }
}
