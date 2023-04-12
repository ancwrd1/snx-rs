//! reverse engineered from vendor snx utility

const TABLE: &[u8] = b"-ODIFIED&W0ROPERTY3HEET7ITH/+4HE3HEET)$3?,$!0?!5?02/0%24)%3.5,,\x10&7?70?/\"*%#43";

fn translate(i: usize, c: u8) -> u8 {
    let mut c = if c == 0xff { 0 } else { c };
    c ^= TABLE[i % 77];

    if c == 0 {
        0xff
    } else {
        c
    }
}

pub fn do_translate<P: AsRef<[u8]>>(data: P) -> Vec<u8> {
    data.as_ref()
        .iter()
        .enumerate()
        .rev()
        .map(|(i, c)| translate(i, *c))
        .collect::<Vec<u8>>()
}

pub fn encode_to_hex<P: AsRef<[u8]>>(data: P) -> String {
    hex::encode(do_translate(data))
}

pub fn decode_from_hex<D: AsRef<[u8]>>(data: D) -> anyhow::Result<Vec<u8>> {
    let mut unhexed = hex::decode(data)?;
    unhexed.reverse();

    let mut decoded = do_translate(unhexed);
    decoded.reverse();

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let username = "testuser";
        let secret = encode_to_hex(username.as_bytes());
        assert_eq!(secret, "36203a333d372a59");

        let decoded = decode_from_hex(secret.as_bytes()).unwrap();
        assert_eq!(decoded, b"testuser");
    }
}
