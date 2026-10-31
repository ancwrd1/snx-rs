use std::ffi::CString;

use interprocess::local_socket::{tokio::Stream, traits::StreamCommon as _};
use nix::unistd::{Group, Uid, User, geteuid, getgrouplist};
use tracing::warn;
use zbus::{Connection, OwnedObjectPath};

const SNX_RS_GROUP: &str = "snx-rs";

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    fn list_sessions(&self) -> zbus::Result<Vec<(String, u32, String, String, OwnedObjectPath)>>;
}

// The command socket lives in the abstract namespace, which has no file permissions, so every peer is
// checked here. Allowed are root, the service's own user, users logged in on a local seat (the equivalent
// of interactive users on Windows and the console user on macOS), and members of the optional snx-rs group
// for setups without logind or for remote logins.
pub(super) async fn authorize_peer(stream: &Stream) -> bool {
    let uid = match stream.peer_creds().map(|creds| creds.euid()) {
        Ok(Some(uid)) => uid,
        Ok(None) => {
            warn!("Rejecting command-socket client with unknown uid");
            return false;
        }
        Err(e) => {
            warn!("Rejecting command-socket client with unreadable credentials: {e}");
            return false;
        }
    };

    if is_privileged(uid, geteuid().as_raw()) || has_seat_session(uid).await || is_in_snx_rs_group(uid).await {
        true
    } else {
        warn!("Rejecting unauthorized command-socket client (uid {uid})");
        false
    }
}

fn is_privileged(uid: u32, service_uid: u32) -> bool {
    uid == 0 || uid == service_uid
}

// A session bound to a seat is a local login; remote ones (ssh) have no seat. logind and elogind
// both provide this API; without either, nobody passes this check.
async fn has_seat_session(uid: u32) -> bool {
    let Ok(connection) = Connection::system().await else {
        return false;
    };
    let Ok(proxy) = Login1ManagerProxy::new(&connection).await else {
        return false;
    };
    let Ok(sessions) = proxy.list_sessions().await else {
        return false;
    };

    is_seated(sessions.iter().map(|(_, uid, _, seat, _)| (*uid, seat.as_str())), uid)
}

fn is_seated<'a>(mut sessions: impl Iterator<Item = (u32, &'a str)>, uid: u32) -> bool {
    sessions.any(|(session_uid, seat)| session_uid == uid && !seat.is_empty())
}

// The peer credentials carry no supplementary groups on Linux, so membership is looked up through NSS,
// which may block on a directory service.
async fn is_in_snx_rs_group(uid: u32) -> bool {
    tokio::task::spawn_blocking(move || {
        let Ok(Some(group)) = Group::from_name(SNX_RS_GROUP) else {
            return false;
        };
        let Ok(Some(user)) = User::from_uid(Uid::from_raw(uid)) else {
            return false;
        };
        let Ok(name) = CString::new(user.name) else {
            return false;
        };
        getgrouplist(&name, user.gid).is_ok_and(|groups| groups.contains(&group.gid))
    })
    .await
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_and_service_user_are_privileged() {
        assert!(is_privileged(0, 0));
        assert!(is_privileged(0, 1000));
        assert!(is_privileged(1000, 1000));
        assert!(!is_privileged(1001, 1000));
        assert!(!is_privileged(1000, 0));
    }

    #[test]
    fn only_sessions_on_a_seat_count() {
        let sessions = [(1000, "seat0"), (1001, ""), (1002, "seat1")];
        assert!(is_seated(sessions.into_iter(), 1000));
        assert!(is_seated(sessions.into_iter(), 1002));
        assert!(!is_seated(sessions.into_iter(), 1001));
        assert!(!is_seated(sessions.into_iter(), 1003));
    }

    #[tokio::test]
    async fn unknown_user_is_not_in_group() {
        assert!(!is_in_snx_rs_group(u32::MAX - 1).await);
    }
}
