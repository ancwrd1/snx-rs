use std::io::{Read, Write};

use anyhow::anyhow;
use snx_rs::util;

fn do_encode() -> anyhow::Result<String> {
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data)?;
    let encoded = util::encode_to_hex(&data);
    Ok(encoded)
}

fn do_decode() -> anyhow::Result<Vec<u8>> {
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data)?;
    let decoded = util::decode_from_hex(&data)?;
    Ok(decoded)
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        return Err(anyhow!(
            "Missing required parameters. Usage: {} {{encode|decode}}",
            args[0]
        ));
    }
    match args[1].as_str() {
        "encode" => Ok(std::io::stdout().write_all(do_encode()?.as_bytes())?),
        "decode" => Ok(std::io::stdout().write_all(&do_decode()?)?),
        _ => Err(anyhow!("Invalid command")),
    }
}
