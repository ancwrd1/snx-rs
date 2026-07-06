use cached::cached;
use tracing::warn;
use uuid::Uuid;

#[cached]
pub fn get_machine_uuid() -> anyhow::Result<Uuid> {
    let mut bytes = [0u8; 16];
    let timeout = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let rc = unsafe { libc::gethostuuid(bytes.as_mut_ptr(), &timeout) };
    if rc != 0 {
        warn!("gethostuuid failed, falling back to nil UUID");
        return Ok(Uuid::nil());
    }
    Ok(Uuid::from_bytes(bytes))
}
