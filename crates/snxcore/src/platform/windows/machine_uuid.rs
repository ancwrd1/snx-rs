use anyhow::anyhow;
use cached::proc_macro::cached;
use tracing::warn;
use uuid::Uuid;

const SUBKEY: &str = r"SOFTWARE\Microsoft\Cryptography";
const VALUE: &str = "MachineGuid";

fn read_machine_guid() -> anyhow::Result<String> {
    let hkey = winreg::HKLM.open_subkey(SUBKEY)?;
    Ok(hkey.get_value(VALUE)?)
}

#[cached(result = true)]
pub fn get_machine_uuid() -> anyhow::Result<Uuid> {
    match read_machine_guid() {
        Ok(s) => Uuid::try_parse(s.trim()).map_err(|e| anyhow!("MachineGuid parse failed: {e}")),
        Err(e) => {
            warn!("Failed to read MachineGuid from registry, falling back to nil UUID: {e}");
            Ok(Uuid::nil())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_uuid() {
        let uuid = get_machine_uuid().unwrap();
        assert_ne!(uuid, Uuid::nil(), "expected a real MachineGuid");
    }
}
