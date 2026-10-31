use std::path::{Component, Path, PathBuf, Prefix};

use windows::{
    Win32::{
        Foundation::{ERROR_SUCCESS, GENERIC_ALL, GENERIC_WRITE, HLOCAL, LocalFree},
        Security::{
            ACCESS_ALLOWED_ACE, ACL,
            Authorization::{ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT},
            DACL_SECURITY_INFORMATION, GetAce, INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
            PSID,
        },
        Storage::FileSystem::{DELETE, FILE_APPEND_DATA, FILE_DELETE_CHILD, FILE_WRITE_DATA, WRITE_DAC, WRITE_OWNER},
        System::SystemServices::{
            ACCESS_ALLOWED_ACE_TYPE, ACCESS_ALLOWED_CALLBACK_ACE_TYPE, ACCESS_DENIED_ACE_TYPE,
            ACCESS_DENIED_CALLBACK_ACE_TYPE,
        },
    },
    core::{HSTRING, PWSTR},
};

const TRUSTED_SIDS: [&str; 3] = [
    // LocalSystem
    "S-1-5-18",
    // BUILTIN\Administrators
    "S-1-5-32-544",
    // NT SERVICE\TrustedInstaller
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
];

// Rights that let a principal replace a library: rewrite it, delete or rename it, or take it over.
const FILE_REPLACE_RIGHTS: u32 =
    FILE_WRITE_DATA.0 | FILE_APPEND_DATA.0 | DELETE.0 | WRITE_DAC.0 | WRITE_OWNER.0 | GENERIC_WRITE.0 | GENERIC_ALL.0;

// Adding entries to a directory is harmless as long as the existing ones cannot be deleted or renamed,
// which is why users may create folders in C:\ without being able to replace anything under it.
const DIR_REPLACE_RIGHTS: u32 = FILE_DELETE_CHILD.0 | DELETE.0 | WRITE_DAC.0 | WRITE_OWNER.0 | GENERIC_ALL.0;

// The service loads the PKCS#11 driver into its own privileged process, so only a driver that no one but
// SYSTEM, administrators or TrustedInstaller can replace is accepted. Bare names are rejected because the
// DLL search order includes PATH, whose directories may be user-writable. The canonical path is returned
// so that a link cannot be swapped between this check and the load.
pub(super) fn trusted_library_path(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }

    let canonical = local_disk_path(&path.canonicalize().ok()?)?;
    let mut ancestors = canonical.ancestors();
    let file = ancestors.next()?;

    (is_secure(file, FILE_REPLACE_RIGHTS) && ancestors.all(|dir| is_secure(dir, DIR_REPLACE_RIGHTS)))
        .then_some(canonical)
}

// Turns \\?\C:\dir into C:\dir. Network shares are rejected, as their ACLs are enforced by another machine.
fn local_disk_path(canonical: &Path) -> Option<PathBuf> {
    let mut components = canonical.components();
    match components.next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::VerbatimDisk(letter) => {
                Some(PathBuf::from(format!("{}:\\", letter as char)).join(components.as_path()))
            }
            _ => None,
        },
        _ => None,
    }
}

fn is_secure(path: &Path, replace_rights: u32) -> bool {
    let mut owner = PSID::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();

    let result = unsafe {
        GetNamedSecurityInfoW(
            &HSTRING::from(path),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            Some(&mut dacl),
            None,
            &mut descriptor,
        )
    };
    if result != ERROR_SUCCESS {
        return false;
    }

    let secure = is_trusted_sid(owner) && !dacl.is_null() && dacl_is_trusted(dacl, replace_rights);
    unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    secure
}

fn dacl_is_trusted(dacl: *const ACL, replace_rights: u32) -> bool {
    let count = unsafe { (*dacl).AceCount };

    (0..u32::from(count)).all(|index| {
        let mut ace = std::ptr::null_mut();
        if unsafe { GetAce(dacl, index, &mut ace) }.is_err() {
            return false;
        }
        // Allowed and denied ACEs, including the callback variants, share the ACCESS_ALLOWED_ACE layout.
        let ace = ace as *const ACCESS_ALLOWED_ACE;
        let (header, mask) = unsafe { ((*ace).Header, (*ace).Mask) };
        let sid = PSID(unsafe { &raw const (*ace).SidStart } as *mut _);

        ace_is_trusted(header.AceType, header.AceFlags, mask, replace_rights, || {
            is_trusted_sid(sid)
        })
    })
}

fn ace_is_trusted(ace_type: u8, flags: u8, mask: u32, replace_rights: u32, trusted_sid: impl FnOnce() -> bool) -> bool {
    match u32::from(ace_type) {
        ACCESS_DENIED_ACE_TYPE | ACCESS_DENIED_CALLBACK_ACE_TYPE => true,
        _ if u32::from(flags) & INHERIT_ONLY_ACE.0 != 0 => true,
        ACCESS_ALLOWED_ACE_TYPE | ACCESS_ALLOWED_CALLBACK_ACE_TYPE => mask & replace_rights == 0 || trusted_sid(),
        _ => false,
    }
}

fn is_trusted_sid(sid: PSID) -> bool {
    let mut string = PWSTR::null();
    if unsafe { ConvertSidToStringSidW(sid, &mut string) }.is_err() {
        return false;
    }
    let trusted = unsafe { string.to_string() }.is_ok_and(|s| TRUSTED_SIDS.contains(&s.as_str()));
    unsafe { LocalFree(Some(HLOCAL(string.0.cast()))) };
    trusted
}

#[cfg(test)]
mod tests {
    use windows::Win32::Storage::FileSystem::{FILE_ADD_SUBDIRECTORY, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ};

    use super::*;

    const ALLOWED: u8 = ACCESS_ALLOWED_ACE_TYPE as u8;
    const DENIED: u8 = ACCESS_DENIED_ACE_TYPE as u8;
    const INHERIT_ONLY: u8 = INHERIT_ONLY_ACE.0 as u8;

    #[test]
    fn read_access_for_anyone_is_trusted() {
        let mask = FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE.0;
        assert!(ace_is_trusted(ALLOWED, 0, mask, FILE_REPLACE_RIGHTS, || false));
    }

    #[test]
    fn write_access_is_trusted_only_for_admins() {
        assert!(!ace_is_trusted(
            ALLOWED,
            0,
            FILE_WRITE_DATA.0,
            FILE_REPLACE_RIGHTS,
            || false
        ));
        assert!(ace_is_trusted(
            ALLOWED,
            0,
            FILE_WRITE_DATA.0,
            FILE_REPLACE_RIGHTS,
            || true
        ));
        assert!(!ace_is_trusted(
            ALLOWED,
            0,
            FILE_DELETE_CHILD.0,
            DIR_REPLACE_RIGHTS,
            || false
        ));
    }

    #[test]
    fn creating_folders_in_a_directory_is_trusted() {
        assert!(ace_is_trusted(
            ALLOWED,
            0,
            FILE_ADD_SUBDIRECTORY.0,
            DIR_REPLACE_RIGHTS,
            || false
        ));
    }

    #[test]
    fn inherit_only_and_deny_aces_are_ignored() {
        assert!(ace_is_trusted(
            ALLOWED,
            INHERIT_ONLY,
            GENERIC_ALL.0,
            FILE_REPLACE_RIGHTS,
            || false
        ));
        assert!(ace_is_trusted(DENIED, 0, GENERIC_ALL.0, FILE_REPLACE_RIGHTS, || false));
    }

    #[test]
    fn unknown_ace_types_are_rejected() {
        assert!(!ace_is_trusted(5, 0, 0, FILE_REPLACE_RIGHTS, || true));
    }

    #[test]
    fn network_and_relative_paths_are_rejected() {
        assert_eq!(trusted_library_path(Path::new("opensc-pkcs11.dll")), None);
        assert_eq!(
            local_disk_path(Path::new(r"\\?\UNC\server\share\opensc-pkcs11.dll")),
            None
        );
        assert_eq!(
            local_disk_path(Path::new(r"\\?\C:\Windows\System32\opensc-pkcs11.dll")),
            Some(PathBuf::from(r"C:\Windows\System32\opensc-pkcs11.dll"))
        );
    }
}
