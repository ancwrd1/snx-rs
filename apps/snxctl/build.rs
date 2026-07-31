use std::{env, error::Error, fs, path::Path};

const RC_TEMPLATE: &str = "assets/snxctl.rc.in";

// See apps/snx-rs/build.rs: the MSI upgrade path needs a versioned binary.
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={RC_TEMPLATE}");

    let version = env!("CARGO_PKG_VERSION");
    let rc = fs::read_to_string(RC_TEMPLATE)?
        .replace("{{app_version}}", version)
        .replace("{{app_version_windows}}", &format!("{},0", version.replace('.', ",")));

    let rc_path = Path::new(&env::var("OUT_DIR")?).join("snxctl.rc");
    fs::write(&rc_path, rc.as_bytes())?;

    embed_resource::compile(rc_path, embed_resource::NONE).manifest_optional()?;

    Ok(())
}
