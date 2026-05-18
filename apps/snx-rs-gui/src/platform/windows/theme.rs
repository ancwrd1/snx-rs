use std::{
    mem,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use tokio::sync::mpsc::Sender;
use tracing::{debug, warn};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Registry::{HKEY, HKEY_CURRENT_USER, KEY_READ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW},
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetMessageW,
            GetWindowLongPtrW, MSG, RegisterClassExW, SetWindowLongPtrW, TranslateMessage, WINDOW_EX_STYLE,
            WM_SETTINGCHANGE, WNDCLASSEXW, WS_POPUP,
        },
    },
    core::w,
};

use crate::platform::TrayCommand;

pub fn spawn_theme_monitor(theme: Arc<AtomicU32>, tray_sender: Sender<TrayCommand>) {
    theme.store(read_system_theme(), Ordering::SeqCst);

    std::thread::Builder::new()
        .name("snx-rs-gui-theme".into())
        .spawn(move || {
            if let Err(e) = run_message_window(theme, tray_sender) {
                warn!("Theme message window exited: {e}");
            }
        })
        .expect("spawn theme thread");
}

fn read_system_theme() -> u32 {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, Some(0), KEY_READ, &mut hkey).is_ok() {
            let mut value: u32 = 0;
            let mut size: u32 = mem::size_of::<u32>() as u32;
            let rc = RegQueryValueExW(
                hkey,
                w!("SystemUsesLightTheme"),
                None,
                None,
                Some(&mut value as *mut u32 as *mut u8),
                Some(&mut size),
            );
            let _ = RegCloseKey(hkey);
            if rc.is_ok() {
                return if value == 0 { 1 } else { 2 };
            }
        }
        0
    }
}

struct WindowState {
    theme: Arc<AtomicU32>,
    tray_sender: Sender<TrayCommand>,
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_SETTINGCHANGE && lparam.0 != 0 {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                if read_wide_str(lparam.0 as *const u16) == "ImmersiveColorSet" {
                    let new_value = read_system_theme();
                    let prev = state.theme.swap(new_value, Ordering::SeqCst);
                    if prev != new_value {
                        debug!("System color scheme: {}", new_value);
                        let sender = state.tray_sender.clone();
                        tokio::spawn(async move { sender.send(TrayCommand::Update(None)).await });
                    }
                }
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe fn read_wide_str(mut ptr: *const u16) -> String {
    unsafe {
        let mut buf = Vec::new();
        while !ptr.is_null() && *ptr != 0 {
            buf.push(*ptr);
            ptr = ptr.add(1);
        }
        String::from_utf16_lossy(&buf)
    }
}

fn run_message_window(theme: Arc<AtomicU32>, tray_sender: Sender<TrayCommand>) -> anyhow::Result<()> {
    unsafe {
        let class_name = w!("SnxRsGuiThemeWindow");
        let hinstance = GetModuleHandleW(None)?;

        let wc = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        let state = Box::new(WindowState { theme, tray_sender });
        let state_ptr = Box::into_raw(state);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("SnxRsGuiThemeWindow"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                drop(Box::from_raw(state_ptr));
                return Err(e.into());
            }
        };

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = DestroyWindow(hwnd);
        drop(Box::from_raw(state_ptr));

        Ok(())
    }
}
