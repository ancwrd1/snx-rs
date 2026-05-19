use anyhow::anyhow;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
        System::Threading::CreateMutexW,
    },
    core::HSTRING,
};

use crate::platform::SingleInstance;

pub struct WindowsSingleInstance {
    handle: HANDLE,
    ok: bool,
}

fn mutex_name(slug: &str) -> String {
    let mut sanitized = String::with_capacity(slug.len());
    for c in slug.chars() {
        let mapped = match c {
            '\\' | '/' | ':' => '_',
            c if c.is_control() => '_',
            c => c,
        };
        if mapped == '_' && sanitized.ends_with('_') {
            continue;
        }
        sanitized.push(mapped);
    }
    let sanitized = sanitized.trim_matches('_');
    format!("Global\\snx-rs-{sanitized}")
}

impl WindowsSingleInstance {
    pub fn new<N: AsRef<str>>(name: N) -> anyhow::Result<Self> {
        let name = HSTRING::from(mutex_name(name.as_ref()));

        unsafe {
            let handle = CreateMutexW(None, false, &name).map_err(|e| anyhow!("CreateMutexW failed: {e}"))?;
            let already = GetLastError() == ERROR_ALREADY_EXISTS;

            Ok(Self { handle, ok: !already })
        }
    }
}

impl Drop for WindowsSingleInstance {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

impl SingleInstance for WindowsSingleInstance {
    fn is_single(&self) -> bool {
        self.ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutex_name_sanitizes_path_separators() {
        assert_eq!(mutex_name("/var/run/snx-rs.lock"), "Global\\snx-rs-var_run_snx-rs.lock");
        assert_eq!(
            mutex_name("C:\\ProgramData\\snx-rs"),
            "Global\\snx-rs-C_ProgramData_snx-rs"
        );
    }

    #[test]
    fn second_instance_is_rejected() {
        let a = WindowsSingleInstance::new("test-snx-rs-single-instance").unwrap();
        assert!(a.is_single());
        let b = WindowsSingleInstance::new("test-snx-rs-single-instance").unwrap();
        assert!(!b.is_single());
    }
}
