use std::sync::{Arc, atomic::AtomicU32};

use tokio::sync::mpsc::Sender;

use crate::platform::TrayCommand;

// macOS tints the tray via template images (see tray.rs), so no runtime theme detection is needed.
pub fn spawn_theme_monitor(_theme: Arc<AtomicU32>, _tray_sender: Sender<TrayCommand>) {}
