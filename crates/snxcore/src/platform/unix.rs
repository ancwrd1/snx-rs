use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

// The service loads the PKCS#11 driver into its own privileged process, so the driver must be one that only
// the service account or root can replace: either a bare file name, which the dynamic loader resolves from
// the system library paths, or a path whose file and every ancestor directory are owned by one of them and
// not writable by group or others. The canonical path is returned so that a symlink cannot be swapped
// between this check and the load.
pub(super) fn trusted_library_path(path: &Path) -> Option<PathBuf> {
    if path.file_name() == Some(path.as_os_str()) {
        return Some(path.to_owned());
    }
    if !path.is_absolute() {
        return None;
    }

    let canonical = path.canonicalize().ok()?;
    let service_uid = nix::unistd::geteuid().as_raw();

    canonical
        .ancestors()
        .all(|p| p.metadata().is_ok_and(|m| is_trusted(m.uid(), m.mode(), service_uid)))
        .then_some(canonical)
}

fn is_trusted(owner: u32, mode: u32, service_uid: u32) -> bool {
    (owner == 0 || owner == service_uid) && mode & 0o022 == 0
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn ownership_and_mode_policy() {
        assert!(is_trusted(0, 0o100755, 1000));
        assert!(is_trusted(1000, 0o100755, 1000));
        assert!(!is_trusted(1001, 0o100755, 1000));
        assert!(!is_trusted(0, 0o100775, 1000));
        assert!(!is_trusted(0, 0o041777, 1000));
    }

    #[test]
    fn bare_name_is_left_to_the_loader() {
        assert_eq!(
            trusted_library_path(Path::new("opensc-pkcs11.so")),
            Some(PathBuf::from("opensc-pkcs11.so"))
        );
    }

    #[test]
    fn relative_paths_are_rejected() {
        assert_eq!(trusted_library_path(Path::new("./opensc-pkcs11.so")), None);
        assert_eq!(trusted_library_path(Path::new("lib/opensc-pkcs11.so")), None);
        assert_eq!(trusted_library_path(Path::new("opensc-pkcs11.so/")), None);
    }

    #[test]
    fn missing_file_is_rejected() {
        assert_eq!(trusted_library_path(Path::new("/nonexistent/opensc-pkcs11.so")), None);
    }

    #[test]
    fn file_in_world_writable_directory_is_rejected() {
        let dir = tempfile::tempdir_in("/tmp").unwrap();
        let lib = dir.path().join("evil.so");
        std::fs::write(&lib, b"").unwrap();
        assert_eq!(trusted_library_path(&lib), None);
    }

    #[test]
    fn system_file_is_accepted_by_canonical_path() {
        let path = Path::new("/bin/sh");
        assert_eq!(trusted_library_path(path), Some(path.canonicalize().unwrap()));
    }
}
