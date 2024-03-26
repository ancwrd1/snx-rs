use std::sync::mpsc;
use std::thread::JoinHandle;

enum BrowserCommand {
    Open(String),
    Exit,
}

pub struct BrowserController {
    sender: mpsc::Sender<BrowserCommand>,
    handle: Option<JoinHandle<()>>,
}

impl BrowserController {
    fn open_browser(url: String) {
        #[cfg(feature = "gui")]
        let _ = crate::gui::webkit::open_browser(url);

        #[cfg(not(feature = "gui"))]
        let _ = opener::open_browser(url);
    }

    fn close_browser() {
        #[cfg(feature = "gui")]
        crate::gui::webkit::close_browser()
    }

    fn run(receiver: mpsc::Receiver<BrowserCommand>) {
        while let Ok(command) = receiver.recv() {
            match command {
                BrowserCommand::Open(url) => Self::open_browser(url),
                BrowserCommand::Exit => break,
            }
        }
    }

    pub fn open<S: AsRef<str>>(&self, url: S) -> anyhow::Result<()> {
        Ok(self.sender.send(BrowserCommand::Open(url.as_ref().to_owned()))?)
    }

    pub fn close(&self) -> anyhow::Result<()> {
        Self::close_browser();
        Ok(())
    }
}

impl Default for BrowserController {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || Self::run(rx));
        Self {
            sender: tx,
            handle: Some(handle),
        }
    }
}

impl Drop for BrowserController {
    fn drop(&mut self) {
        let _ = self.sender.send(BrowserCommand::Exit);
        let _ = self.handle.take().map(|h| h.join());
    }
}
