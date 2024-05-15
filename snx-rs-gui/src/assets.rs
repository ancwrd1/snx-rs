use std::io;

use once_cell::sync::Lazy;

use crate::theme::system_color_theme;

fn png_to_argb(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decoder = png::Decoder::new(io::Cursor::new(data));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];

    let info = reader.next_frame(&mut buf)?;
    let bytes = buf[..info.buffer_size()].to_vec();

    Ok(bytes)
}

pub struct IconTheme {
    pub vpn: Vec<u8>,
    pub acquiring: Vec<u8>,
    pub error: Vec<u8>,
    pub disconnected: Vec<u8>,
    pub connected: Vec<u8>,
}

static DARK_THEME_ARGB: Lazy<IconTheme> = Lazy::new(|| IconTheme {
    vpn: png_to_argb(include_bytes!("../../assets/icons/dark/network-vpn.png")).unwrap_or_default(),
    acquiring: png_to_argb(include_bytes!("../../assets/icons/dark/network-vpn-acquiring.png")).unwrap_or_default(),
    error: png_to_argb(include_bytes!("../../assets/icons/dark/network-vpn-error.png")).unwrap_or_default(),
    disconnected: png_to_argb(include_bytes!("../../assets/icons/dark/network-vpn-disconnected.png"))
        .unwrap_or_default(),
    connected: png_to_argb(include_bytes!("../../assets/icons/dark/network-vpn-connected.png")).unwrap_or_default(),
});

static LIGHT_THEME_ARGB: Lazy<IconTheme> = Lazy::new(|| IconTheme {
    vpn: png_to_argb(include_bytes!("../../assets/icons/light/network-vpn.png")).unwrap_or_default(),
    acquiring: png_to_argb(include_bytes!("../../assets/icons/light/network-vpn-acquiring.png")).unwrap_or_default(),
    error: png_to_argb(include_bytes!("../../assets/icons/light/network-vpn-error.png")).unwrap_or_default(),
    disconnected: png_to_argb(include_bytes!("../../assets/icons/light/network-vpn-disconnected.png"))
        .unwrap_or_default(),
    connected: png_to_argb(include_bytes!("../../assets/icons/light/network-vpn-connected.png")).unwrap_or_default(),
});

pub fn current_icon_theme() -> &'static IconTheme {
    if system_color_theme().unwrap_or_default().is_dark() {
        &DARK_THEME_ARGB
    } else {
        &LIGHT_THEME_ARGB
    }
}
