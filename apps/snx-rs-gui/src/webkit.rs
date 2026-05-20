use std::{process::Stdio, sync::Arc, time::Duration};

use i18n::tr;
use snxcore::{
    browser::{BrowserController, SystemBrowser},
    model::params::TunnelParams,
};

const PASSWORD_TIMEOUT: Duration = Duration::from_secs(120);

pub const JS_PASSWORD_SCRIPT: &str = r#"
(function() {
  const regexes = [
    /sPropertyName = "password";\n\s*SNXParams\.addProperty\(sPropertyName, Function\.READ_WRITE, "([^"]+)"\);/,
    /Extender\.password\s*=\s*"([^"]+)"/,
  ];

  const scripts = document.querySelectorAll("script:not([src])");
  for (const s of scripts) {
    for (const regex of regexes) {
      const match = s.textContent.match(regex);
      if (match) return match[1];
    }
  }

  return "";
})();
"#;

pub struct WebKitBrowser {
    params: Arc<TunnelParams>,
}

impl WebKitBrowser {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self { params }
    }
}

impl BrowserController for WebKitBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        SystemBrowser::default().open(url)
    }

    fn close(&self) {}

    async fn acquire_tunnel_password(&self, url: &str) -> anyhow::Result<String> {
        let exe = std::env::current_exe()?;

        let mut cmd = tokio::process::Command::new(exe);
        cmd.arg("--webkit").arg(url);
        if self.params.ignore_server_cert {
            cmd.arg("--webkit-ignore-cert");
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let output = tokio::time::timeout(PASSWORD_TIMEOUT, cmd.output()).await;

        if let Ok(Ok(output)) = output
            && output.status.success()
        {
            let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !password.is_empty() {
                return Ok(password);
            }
        }

        anyhow::bail!(tr!("error-cannot-acquire-access-cookie"))
    }
}
