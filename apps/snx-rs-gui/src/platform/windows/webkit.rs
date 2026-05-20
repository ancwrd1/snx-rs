use std::{cell::RefCell, process::Stdio, rc::Rc, sync::Arc, time::Duration};

use i18n::tr;
use snxcore::{
    browser::{BrowserController, SystemBrowser},
    model::params::TunnelParams,
};
use tracing::warn;
use webview2_com::{
    CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler,
    ExecuteScriptCompletedHandler,
    Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW, CreateCoreWebView2EnvironmentWithOptions,
        ICoreWebView2, ICoreWebView2_14, ICoreWebView2Controller, ICoreWebView2Environment,
        ICoreWebView2EnvironmentOptions,
    },
    NavigationCompletedEventHandler, ServerCertificateErrorDetectedEventHandler,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::{
            Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
            LibraryLoader::GetModuleHandleW,
        },
        UI::{
            HiDpi::{GetDpiForSystem, PROCESS_PER_MONITOR_DPI_AWARE, SetProcessDpiAwareness},
            WindowsAndMessaging::{
                CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GWLP_USERDATA, GetClientRect,
                GetMessageW, GetWindowLongPtrW, IDC_ARROW, LoadCursorW, MSG, PostQuitMessage, RegisterClassW, SW_SHOW,
                SetWindowLongPtrW, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_DESTROY, WM_SIZE, WNDCLASSW,
                WS_OVERLAPPEDWINDOW,
            },
        },
    },
    core::{HSTRING, Interface, PCWSTR, w},
};

const PASSWORD_TIMEOUT: Duration = Duration::from_secs(120);
const WINDOW_CLASS: PCWSTR = w!("SnxRsWebViewWindow");

const JS_PASSWORD_SCRIPT: &str = r#"
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

struct WebKitState {
    controller: RefCell<Option<ICoreWebView2Controller>>,
    password: RefCell<Option<String>>,
}

pub fn webkit_main(url: &str, ignore_cert: bool) -> i32 {
    let _ = unsafe { SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE) };

    // WebView2 requires STA on the UI thread.
    if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_err() {
        warn!("CoInitializeEx failed");
        return 1;
    }

    let exit_code = run(url, ignore_cert).unwrap_or_else(|e| {
        warn!("webkit_main: {e:#}");
        1
    });

    unsafe { CoUninitialize() };
    exit_code
}

fn run(url: &str, ignore_cert: bool) -> anyhow::Result<i32> {
    let hwnd = create_window()?;

    let state = Rc::new(WebKitState {
        controller: RefCell::new(None),
        password: RefCell::new(None),
    });

    let state_ptr = Rc::as_ptr(&state) as isize;
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr) };

    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };

    let user_data_dir = tempfile::tempdir()?;
    create_webview(hwnd, url, ignore_cert, user_data_dir.path(), state.clone())?;

    spawn_timeout_thread(hwnd);
    pump_messages(hwnd);

    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };

    match state.password.borrow().as_ref() {
        Some(p) => {
            println!("{p}");
            Ok(0)
        }
        None => Ok(1),
    }
}

fn create_window() -> anyhow::Result<HWND> {
    let hinstance = unsafe { GetModuleHandleW(None) }?;
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }?;

    let wc = WNDCLASSW {
        hInstance: hinstance.into(),
        lpszClassName: WINDOW_CLASS,
        lpfnWndProc: Some(wnd_proc),
        hCursor: cursor,
        ..Default::default()
    };
    unsafe { RegisterClassW(&wc) };

    let title = HSTRING::from(tr!("label-mobile-access"));
    // Process is PROCESS_PER_MONITOR_DPI_AWARE, so CreateWindowExW takes
    // physical pixels. The WebView2 content scales with monitor DPI, so we
    // scale the window by the same factor — otherwise at 200% the window
    // gets ~half the room the content expects and looks cropped.
    let dpi = unsafe { GetDpiForSystem() } as i32;
    let width = 900 * dpi / 96;
    let height = 650 * dpi / 96;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            WINDOW_CLASS,
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
    }?;

    Ok(hwnd)
}

fn create_webview(
    hwnd: HWND,
    url: &str,
    ignore_cert: bool,
    user_data_dir: &std::path::Path,
    state: Rc<WebKitState>,
) -> anyhow::Result<()> {
    let url = url.to_owned();

    let environment = wait_env(user_data_dir)?;

    let controller = wait_controller(&environment, hwnd)?;

    let mut rc = RECT::default();

    unsafe {
        GetClientRect(hwnd, &mut rc)?;
        controller.SetBounds(rc)?;
        controller.SetIsVisible(true)?;
    }

    *state.controller.borrow_mut() = Some(controller.clone());

    let webview: ICoreWebView2 = unsafe { controller.CoreWebView2() }?;

    if ignore_cert {
        install_ignore_cert_handler(&webview)?;
    }

    install_navigation_completed(&webview, state.clone())?;

    let url_w = HSTRING::from(url);
    unsafe { webview.Navigate(PCWSTR(url_w.as_ptr())) }?;

    Ok(())
}

fn wait_env(user_data_dir: &std::path::Path) -> anyhow::Result<ICoreWebView2Environment> {
    let slot: Rc<RefCell<Option<ICoreWebView2Environment>>> = Rc::new(RefCell::new(None));

    let slot_cb = slot.clone();
    let user_data_dir = HSTRING::from(user_data_dir.as_os_str());

    CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                PCWSTR(user_data_dir.as_ptr()),
                None::<&ICoreWebView2EnvironmentOptions>,
                &handler,
            )
            .map_err(Into::into)
        }),
        Box::new(move |error_code, environment| {
            error_code?;
            *slot_cb.borrow_mut() = environment;
            Ok(())
        }),
    )
    .map_err(|e| anyhow::anyhow!("CreateCoreWebView2EnvironmentWithOptions failed: {e}"))?;

    slot.borrow_mut()
        .take()
        .ok_or_else(|| anyhow::anyhow!("WebView2 environment not created"))
}

fn wait_controller(env: &ICoreWebView2Environment, hwnd: HWND) -> anyhow::Result<ICoreWebView2Controller> {
    let slot: Rc<RefCell<Option<ICoreWebView2Controller>>> = Rc::new(RefCell::new(None));
    let slot_cb = slot.clone();
    let env_for_cb = env.clone();
    let hwnd_raw = hwnd.0 as isize;

    CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            env_for_cb
                .CreateCoreWebView2Controller(HWND(hwnd_raw as _), &handler)
                .map_err(Into::into)
        }),
        Box::new(move |error_code, controller| {
            error_code?;
            *slot_cb.borrow_mut() = controller;
            Ok(())
        }),
    )
    .map_err(|e| anyhow::anyhow!("CreateCoreWebView2Controller failed: {e}"))?;

    slot.borrow_mut()
        .take()
        .ok_or_else(|| anyhow::anyhow!("WebView2 controller not created"))
}

fn install_ignore_cert_handler(webview: &ICoreWebView2) -> anyhow::Result<()> {
    let v14: ICoreWebView2_14 = match webview.cast() {
        Ok(v) => v,
        Err(e) => {
            warn!("WebView2 too old for ServerCertificateErrorDetected ({e}); ignore_cert ignored");
            return Ok(());
        }
    };

    let handler = ServerCertificateErrorDetectedEventHandler::create(Box::new(|_sender, args| {
        if let Some(args) = args {
            unsafe { args.SetAction(COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW) }?;
        }
        Ok(())
    }));

    let mut token = Default::default();
    unsafe { v14.add_ServerCertificateErrorDetected(&handler, &mut token) }?;

    Ok(())
}

fn install_navigation_completed(webview: &ICoreWebView2, state: Rc<WebKitState>) -> anyhow::Result<()> {
    let webview_for_js = webview.clone();
    let handler = NavigationCompletedEventHandler::create(Box::new(move |_sender, _args| {
        let state = state.clone();
        let webview = webview_for_js.clone();
        let script = HSTRING::from(JS_PASSWORD_SCRIPT);

        let result_handler = ExecuteScriptCompletedHandler::create(Box::new(move |error_code, json_result| {
            if error_code.is_err() {
                return Ok(());
            }
            let s = json_result;
            let trimmed = s.trim_matches('"');
            if !trimmed.is_empty() {
                *state.password.borrow_mut() = Some(trimmed.to_string());
                unsafe { PostQuitMessage(0) };
            }
            Ok(())
        }));

        unsafe { webview.ExecuteScript(PCWSTR(script.as_ptr()), &result_handler) }?;

        Ok(())
    }));

    let mut token = Default::default();
    unsafe { webview.add_NavigationCompleted(&handler, &mut token) }?;

    Ok(())
}

fn pump_messages(_hwnd: HWND) {
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn spawn_timeout_thread(hwnd: HWND) {
    let hwnd_raw = hwnd.0 as isize;
    std::thread::spawn(move || {
        std::thread::sleep(PASSWORD_TIMEOUT);
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
            let _ = PostMessageW(Some(HWND(hwnd_raw as _)), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    });
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if state_ptr != 0 {
                let state = unsafe { &*(state_ptr as *const WebKitState) };
                if let Some(controller) = state.controller.borrow().as_ref() {
                    let mut rc = RECT::default();
                    if unsafe { GetClientRect(hwnd, &mut rc) }.is_ok() {
                        let _ = unsafe { controller.SetBounds(rc) };
                    }
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

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
