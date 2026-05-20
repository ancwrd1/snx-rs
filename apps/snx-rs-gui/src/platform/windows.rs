#![allow(unsafe_code)]

pub mod theme;
pub mod tray;
#[cfg(feature = "mobile-access")]
pub mod webkit;

use std::{env, mem};

use tauri_winrt_notification::Toast;
use tracing::{debug, warn};
#[cfg(feature = "mobile-access")]
pub use webkit::webkit_main;
use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::{
            Gdi::{GetDC, ReleaseDC},
            OpenGL::{
                ChoosePixelFormat, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL,
                PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR, SetPixelFormat, wglCreateContext, wglDeleteContext,
                wglGetProcAddress, wglMakeCurrent,
            },
        },
        System::LibraryLoader::LoadLibraryW,
        UI::WindowsAndMessaging::{
            CreateWindowExA, DefWindowProcA, DestroyWindow, RegisterClassA, UnregisterClassA, WINDOW_EX_STYLE,
            WNDCLASSA, WNDPROC, WS_OVERLAPPED,
        },
    },
    core::{s, w},
};

pub async fn send_notification(summary: &str, message: &str) -> anyhow::Result<()> {
    Ok(Toast::new("com.github.snx-rs")
        .title(summary)
        .text1(message)
        .duration(tauri_winrt_notification::Duration::Short)
        .show()?)
}

pub fn user_tag() -> String {
    let raw = env::var("USERNAME").unwrap_or_else(|_| "user".into());
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn init_gui_backend() -> anyhow::Result<()> {
    if let Ok(backend) = env::var("SLINT_BACKEND") {
        debug!("Using pre-selected rendering backend: {backend}");
        slint::BackendSelector::new().select()?;
        return Ok(());
    }

    let renderer = if has_opengl_support() { "femtovg" } else { "software" };
    debug!("Using renderer: {renderer}");

    if let Err(err) = slint::BackendSelector::new().renderer_name(renderer.into()).select() {
        warn!("Failed to select rendering backend {renderer}: {err}; falling back to default");
        slint::BackendSelector::new().select()?;
    }
    Ok(())
}

fn has_opengl_support() -> bool {
    unsafe {
        if LoadLibraryW(w!("opengl32.dll")).is_err() {
            return false;
        }
        probe_gl_function()
    }
}

unsafe fn probe_gl_function() -> bool {
    unsafe {
        let class_name = s!("SnxRsGuiGLProbe");
        let wc = WNDCLASSA {
            lpfnWndProc: mem::transmute::<usize, WNDPROC>(DefWindowProcA as *const () as usize),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassA(&wc);

        let hwnd = CreateWindowExA(
            WINDOW_EX_STYLE(0),
            class_name,
            s!(""),
            WS_OVERLAPPED,
            0,
            0,
            1,
            1,
            Some(HWND::default()),
            None,
            None,
            None,
        );
        let Ok(hwnd) = hwnd else { return false };
        if hwnd.is_invalid() {
            return false;
        }

        let hdc = GetDC(Some(hwnd));
        let mut result = false;

        let pfd = PIXELFORMATDESCRIPTOR {
            nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cDepthBits: 24,
            iLayerType: PFD_MAIN_PLANE.0 as u8,
            ..Default::default()
        };

        let pixel_format = ChoosePixelFormat(hdc, &pfd);
        if pixel_format != 0
            && SetPixelFormat(hdc, pixel_format, &pfd).is_ok()
            && let Ok(hglrc) = wglCreateContext(hdc)
        {
            if wglMakeCurrent(hdc, hglrc).is_ok() {
                let ptr = wglGetProcAddress(s!("glCreateShader"));
                // wglGetProcAddress returns 1/2/3/-1 as "not found" sentinels on
                // some drivers in addition to NULL. Treat all of those as missing.
                result = match ptr.map(|f| f as usize) {
                    None | Some(1) | Some(2) | Some(3) | Some(usize::MAX) => false,
                    Some(_) => true,
                };
                let _ = wglMakeCurrent(hdc, HGLRC::default());
            }
            let _ = wglDeleteContext(hglrc);
        }

        ReleaseDC(Some(hwnd), hdc);
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassA(class_name, None);

        result
    }
}

pub async fn wait_restart_signal() -> anyhow::Result<()> {
    std::future::pending::<()>().await;
    Ok(())
}
