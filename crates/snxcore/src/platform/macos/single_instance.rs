use std::{fs, os::fd::OwnedFd};

use nix::{
    fcntl::{self, Flock, FlockArg, OFlag},
    sys::stat::Mode,
};

use crate::platform::SingleInstance;

pub struct MacosSingleInstance {
    name: String,
    handle: Option<Flock<OwnedFd>>,
}

impl MacosSingleInstance {
    pub fn new<N: AsRef<str>>(name: N) -> anyhow::Result<Self> {
        let fd = fcntl::open(
            name.as_ref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::from_bits_truncate(0o600),
        )?;

        let handle = Flock::lock(fd, FlockArg::LockExclusiveNonblock).ok();

        Ok(Self {
            name: name.as_ref().to_owned(),
            handle,
        })
    }
}

impl Drop for MacosSingleInstance {
    fn drop(&mut self) {
        if self.handle.take().is_some() {
            let _ = fs::remove_file(&self.name);
        }
    }
}

impl SingleInstance for MacosSingleInstance {
    fn is_single(&self) -> bool {
        self.handle.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_is_rejected() {
        let path = std::env::temp_dir().join(format!("snx-rs-test-single-instance-{}.lock", std::process::id()));
        let path = path.to_str().unwrap();

        let a = MacosSingleInstance::new(path).unwrap();
        assert!(a.is_single());

        let b = MacosSingleInstance::new(path).unwrap();
        assert!(!b.is_single());

        drop(a);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn lock_is_reacquirable_after_drop() {
        let path = std::env::temp_dir().join(format!("snx-rs-test-si-drop-{}.lock", std::process::id()));
        let path = path.to_str().unwrap();

        {
            let a = MacosSingleInstance::new(path).unwrap();
            assert!(a.is_single());
        }
        let c = MacosSingleInstance::new(path).unwrap();
        assert!(c.is_single(), "lock must be free once the previous holder drops");

        drop(c);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn lock_file_is_private_to_the_user() {
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join(format!("snx-rs-test-si-perms-{}.lock", std::process::id()));
        let a = MacosSingleInstance::new(path.to_str().unwrap()).unwrap();
        assert!(a.is_single());

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "lock file must not be group/world accessible");

        drop(a);
    }
}
