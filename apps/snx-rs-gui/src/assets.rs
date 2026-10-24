use std::sync::LazyLock;

use resvg::{tiny_skia, usvg};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

pub const ICON_SIZE: u32 = 256;

const SYSTEM_ICONS_DIR: &str = "/usr/share/icons/hicolor/symbolic/apps";

const ACQUIRING_SVG: &str = include_str!("../../../package/icons/snx-rs-acquiring-symbolic.svg");
const CONNECTED_SVG: &str = include_str!("../../../package/icons/snx-rs-connected-symbolic.svg");
const DISCONNECTED_SVG: &str = include_str!("../../../package/icons/snx-rs-disconnected-symbolic.svg");
const ERROR_SVG: &str = include_str!("../../../package/icons/snx-rs-error-symbolic.svg");

struct ThemeColors {
    text: &'static str,
    dim_opacity: &'static str,
}

const DARK_BG_COLORS: ThemeColors = ThemeColors {
    text: "#eff0f1",
    dim_opacity: "0.6",
};

const LIGHT_BG_COLORS: ThemeColors = ThemeColors {
    text: "#232629",
    dim_opacity: "0.4",
};

fn render_pixmap(svg: &str, colors: &ThemeColors, size: u32) -> Option<tiny_skia::Pixmap> {
    let themed = svg
        .replace("#232629", colors.text)
        .replace(r#"opacity="0.3""#, &format!(r#"opacity="{}""#, colors.dim_opacity));

    let tree = usvg::Tree::from_str(&themed, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;

    let svg_size = tree.size();
    let scale = size as f32 / svg_size.width().max(svg_size.height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

fn render_rgba(svg: &str, colors: &ThemeColors, size: u32) -> Vec<u8> {
    render_pixmap(svg, colors, size).map(|p| p.take()).unwrap_or_default()
}

fn render_image(svg: &str, colors: &ThemeColors, size: u32) -> Image {
    let Some(pixmap) = render_pixmap(svg, colors, size) else {
        return Image::default();
    };

    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(pixmap.data(), size, size);
    Image::from_rgba8_premultiplied(buffer)
}

pub struct IconTheme {
    pub acquiring: Vec<u8>,
    pub error: Vec<u8>,
    pub disconnected: Vec<u8>,
    pub connected: Vec<u8>,
}

impl IconTheme {
    fn render(colors: &ThemeColors) -> Self {
        Self {
            acquiring: render_rgba(ACQUIRING_SVG, colors, ICON_SIZE),
            error: render_rgba(ERROR_SVG, colors, ICON_SIZE),
            disconnected: render_rgba(DISCONNECTED_SVG, colors, ICON_SIZE),
            connected: render_rgba(CONNECTED_SVG, colors, ICON_SIZE),
        }
    }
}

pub static DARK_THEME: LazyLock<IconTheme> = LazyLock::new(|| IconTheme::render(&DARK_BG_COLORS));
pub static LIGHT_THEME: LazyLock<IconTheme> = LazyLock::new(|| IconTheme::render(&LIGHT_BG_COLORS));

pub fn connected_icon_for_dark_bg() -> Image {
    render_image(CONNECTED_SVG, &DARK_BG_COLORS, ICON_SIZE)
}

pub fn connected_icon_for_light_bg() -> Image {
    render_image(CONNECTED_SVG, &LIGHT_BG_COLORS, ICON_SIZE)
}

pub fn system_icons_installed() -> bool {
    std::fs::read_dir(SYSTEM_ICONS_DIR)
        .map(|r| {
            r.flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with("snx-rs"))
        })
        .unwrap_or(false)
}
