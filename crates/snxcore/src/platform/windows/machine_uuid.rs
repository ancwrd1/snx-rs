use anyhow::anyhow;
use cached::proc_macro::cached;
use tracing::warn;
use uuid::Uuid;
use windows::{
    Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, REG_VALUE_TYPE, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    },
    core::HSTRING,
};

const SUBKEY: &str = r"SOFTWARE\Microsoft\Cryptography";
const VALUE: &str = "MachineGuid";

fn read_machine_guid() -> anyhow::Result<String> {
    let subkey = HSTRING::from(SUBKEY);
    let value = HSTRING::from(VALUE);

    let mut hkey = HKEY::default();
    unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, &subkey, None, KEY_READ, &mut hkey) }
        .ok()
        .map_err(|e| anyhow!("RegOpenKeyExW failed: {e}"))?;

    let result = (|| {
        let mut kind = REG_VALUE_TYPE::default();
        let mut len: u32 = 0;

        unsafe { RegQueryValueExW(hkey, &value, None, Some(&mut kind), None, Some(&mut len)) }
            .ok()
            .map_err(|e| anyhow!("RegQueryValueExW (size) failed: {e}"))?;

        if kind != REG_SZ {
            return Err(anyhow!("MachineGuid has unexpected type {:?}", kind.0));
        }

        let mut buf = vec![0u8; len as usize];
        let mut actual_len = len;
        unsafe {
            RegQueryValueExW(
                hkey,
                &value,
                None,
                Some(&mut kind),
                Some(buf.as_mut_ptr()),
                Some(&mut actual_len),
            )
        }
        .ok()
        .map_err(|e| anyhow!("RegQueryValueExW (read) failed: {e}"))?;

        buf.truncate(actual_len as usize);

        let wide: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();

        Ok(String::from_utf16(&wide)?)
    })();

    let _ = unsafe { RegCloseKey(hkey) };

    result
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
