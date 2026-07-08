use std::{
    io,
    path::{Path, PathBuf},
};

use interprocess::local_socket::{GenericFilePath, Name, ToFsName, tokio::Stream, traits::StreamCommon as _};
use tracing::warn;

use crate::server::DEFAULT_NAME;

// The generic namespaced name maps to a world-writable path under /tmp on macOS; pin the socket to a
// filesystem path instead so the directory it lives in cannot be squatted.
pub(super) fn name(name: &str) -> io::Result<Name<'static>> {
    path(name).to_fs_name::<GenericFilePath>()
}

fn path(name: &str) -> PathBuf {
    if name.starts_with('/') {
        PathBuf::from(name)
    } else if name == DEFAULT_NAME {
        // The privileged daemon's well-known socket lives in the root-owned /var/run.
        Path::new("/var/run").join(name)
    } else {
        // Custom, per-user sockets (tests, non-default instances) go to the user temp dir, which does
        // not require root and lets a same-user client and server agree on the path.
        std::env::temp_dir().join(name)
    }
}

// interprocess creates the socket 0755, blocking the unprivileged GUI. Widen to 0666: the mode only
// governs who may connect, not who is served - each peer is then authenticated by authorize_peer, and
// the root-owned /var/run cannot be hijacked.
pub(super) fn secure(name: &str) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path(name), std::fs::Permissions::from_mode(0o666))
}

// The root daemon only acts for authorized local peers: root, the interactive console user or admins.
pub(super) fn authorize_peer(stream: &Stream) -> bool {
    match stream.peer_creds() {
        Ok(creds) if is_authorized(creds.euid(), creds.groups(), console_user_uid()) => true,
        Ok(creds) => {
            warn!("Rejecting unauthorized command-socket client (uid {:?})", creds.euid());
            false
        }
        Err(e) => {
            warn!("Rejecting command-socket client with unreadable credentials: {e}");
            false
        }
    }
}

// uid of the interactive console user (owner of /dev/console), i.e. whoever runs the GUI.
fn console_user_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/dev/console").ok().map(|m| m.uid())
}

// Mirror the Windows security descriptor (SYSTEM + Administrators + interactive users): allow root,
// the console user and admins. A pure function of the peer credentials so it can be unit-tested.
fn is_authorized(euid: Option<u32>, groups: Option<&[u32]>, console_uid: Option<u32>) -> bool {
    const ADMIN_GID: u32 = 80;
    match euid {
        Some(uid) => uid == 0 || Some(uid) == console_uid || groups.is_some_and(|g| g.contains(&ADMIN_GID)),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn path_routes_by_name() {
        // The well-known daemon socket is pinned to the root-owned /var/run.
        assert_eq!(
            super::path(super::DEFAULT_NAME),
            std::path::Path::new("/var/run").join(super::DEFAULT_NAME)
        );
        // Absolute names are used verbatim.
        assert_eq!(
            super::path("/tmp/custom.sock"),
            std::path::PathBuf::from("/tmp/custom.sock")
        );
        // Custom relative names go to the user-writable temp dir, not /var/run.
        assert_eq!(
            super::path("snxcore-test.sock"),
            std::env::temp_dir().join("snxcore-test.sock")
        );
    }

    #[test]
    fn name_builds() {
        assert!(super::name("snx-rs.sock").is_ok());
    }

    #[test]
    fn authorization_policy() {
        const CONSOLE: Option<u32> = Some(501);
        // root is always allowed.
        assert!(super::is_authorized(Some(0), None, CONSOLE));
        // the interactive console user (running the GUI) is allowed.
        assert!(super::is_authorized(Some(501), Some(&[20, 12]), CONSOLE));
        // a member of the admin group (80) is allowed even when not the console user.
        assert!(super::is_authorized(Some(502), Some(&[20, 80]), CONSOLE));
        // an unrelated non-admin local user is rejected.
        assert!(!super::is_authorized(Some(502), Some(&[20, 12]), CONSOLE));
        // unreadable credentials fail closed.
        assert!(!super::is_authorized(None, None, CONSOLE));
        // with no known console user, only root and admins get in.
        assert!(super::is_authorized(Some(0), None, None));
        assert!(!super::is_authorized(Some(501), Some(&[20, 12]), None));
    }
}
