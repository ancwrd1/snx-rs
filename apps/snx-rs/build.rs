use std::{env, error::Error, fs, path::Path};

const RC_TEMPLATE: &str = "assets/snx-rs.rc.in";

// The MSI installer relies on the VERSIONINFO resource: without it MSI treats
// snx-rs.exe as an unversioned file and refuses to replace it during a major
// upgrade.
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={RC_TEMPLATE}");

    let version = env!("CARGO_PKG_VERSION");
    let rc = fs::read_to_string(RC_TEMPLATE)?
        .replace("{{app_version}}", version)
        .replace("{{app_version_windows}}", &format!("{},0", version.replace('.', ",")));

    let rc_path = Path::new(&env::var("OUT_DIR")?).join("snx-rs.rc");
    fs::write(&rc_path, rc.as_bytes())?;

    embed_resource::compile(rc_path, embed_resource::NONE).manifest_optional()?;

    Ok(())
}
