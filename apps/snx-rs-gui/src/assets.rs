use std::{io, sync::LazyLock};

use anyhow::Context;

fn png_to_argb(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decoder = png::Decoder::new(io::Cursor::new(data));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size().context("Failed to read PNG info")?];

    let info = reader.next_frame(&mut buf)?;
    let mut bytes = buf[..info.buffer_size()].to_vec();

    for pixel in bytes.chunks_exact_mut(4) {
        pixel.rotate_right(1);
    }

    Ok(bytes)
}

macro_rules! argb {
    ($path:literal) => {
        png_to_argb(include_bytes!($path)).unwrap_or_default()
    };
}

pub struct IconTheme {
    pub acquiring: Vec<u8>,
    pub error: Vec<u8>,
    pub disconnected: Vec<u8>,
    pub connected: Vec<u8>,
}

pub static DARK_THEME: LazyLock<IconTheme> = LazyLock::new(|| IconTheme {
    acquiring: argb!("../assets/icons/dark/network-vpn-acquiring.png"),
    error: argb!("../assets/icons/dark/network-vpn-error.png"),
    disconnected: argb!("../assets/icons/dark/network-vpn-disconnected.png"),
    connected: argb!("../assets/icons/dark/network-vpn-connected.png"),
});

pub static LIGHT_THEME: LazyLock<IconTheme> = LazyLock::new(|| IconTheme {
    acquiring: argb!("../assets/icons/light/network-vpn-acquiring.png"),
    error: argb!("../assets/icons/light/network-vpn-error.png"),
    disconnected: argb!("../assets/icons/light/network-vpn-disconnected.png"),
    connected: argb!("../assets/icons/light/network-vpn-connected.png"),
});

pub const APP_CSS: &str = include_str!("../assets/app.css");
