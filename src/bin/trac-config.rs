use anyhow::anyhow;
use snx_rs::util;

fn do_encode(filename: &str) -> anyhow::Result<()> {
    let data = std::fs::read(filename)?;
    let encoded = util::encode_to_hex(data);
    print!("11TRAC{}", encoded);
    Ok(())
}

fn do_decode(filename: &str) -> anyhow::Result<()> {
    let data = std::fs::read(filename)?;
    if data.starts_with(b"11TRAC") {
        let decoded = util::decode_from_hex(&data[6..])?;
        print!("{}", String::from_utf8_lossy(&decoded));
        Ok(())
    } else {
        Err(anyhow!("Not a Checkpoint TRAC config file!"))
    }
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        return Err(anyhow!(
            "Missing required parameters. Usage: {} {{encode|decode}} filename",
            args[0]
        ));
    }
    match args[1].as_str() {
        "encode" => do_encode(&args[2]),
        "decode" => do_decode(&args[2]),
        _ => Err(anyhow!("Invalid command")),
    }
}
