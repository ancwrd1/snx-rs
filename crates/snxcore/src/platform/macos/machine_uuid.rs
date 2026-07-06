use cached::cached;
use uuid::Uuid;

// #[cached] skips caching Err, so a transient failure returns an error (the caller falls back to a
// random UUID) instead of caching a fixed nil UUID for the process's life.
#[cached]
pub fn get_machine_uuid() -> anyhow::Result<Uuid> {
    let mut bytes = [0u8; 16];
    let timeout = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let rc = unsafe { libc::gethostuuid(bytes.as_mut_ptr(), &timeout) };
    if rc != 0 {
        anyhow::bail!("gethostuuid failed: {}", std::io::Error::last_os_error());
    }
    Ok(Uuid::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    #[test]
    fn machine_uuid_is_stable_and_non_nil() {
        let first = super::get_machine_uuid().expect("gethostuuid must succeed");
        let second = super::get_machine_uuid().expect("gethostuuid must succeed");
        assert_eq!(first, second, "host UUID must be stable across calls");
        assert!(!first.is_nil(), "expected a real host UUID on macOS hardware");
    }
}
