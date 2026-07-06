use std::sync::atomic::{AtomicBool, Ordering};

use block2::RcBlock;
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSError, NSNotification, NSPoint, NSRect, NSSize, NSString, NSURL, NSURLRequest,
};
use objc2_web_kit::{WKNavigation, WKNavigationDelegate, WKWebView, WKWebViewConfiguration, WKWebsiteDataStore};

const PASSWORD_TIMEOUT_SECS: u64 = 120;
const WINDOW_WIDTH: f64 = 900.0;
const WINDOW_HEIGHT: f64 = 650.0;

// The JS success handler, the timeout, and closing the window all end the process; the first one
// wins so the exit code is unambiguous and the rest become no-ops.
static EXITED: AtomicBool = AtomicBool::new(false);

fn exit_once(code: i32) {
    if !EXITED.swap(true, Ordering::SeqCst) {
        std::process::exit(code);
    }
}

struct DelegateIvars {
    webview: Retained<WKWebView>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SnxRsWebkitDelegate"]
    #[ivars = DelegateIvars]
    struct WebkitDelegate;

    unsafe impl NSObjectProtocol for WebkitDelegate {}

    unsafe impl WKNavigationDelegate for WebkitDelegate {
        #[unsafe(method(webView:didFinishNavigation:))]
        fn did_finish(&self, _web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            let script = NSString::from_str(crate::webkit::JS_PASSWORD_SCRIPT);
            let completion = RcBlock::new(|result: *mut AnyObject, _error: *mut NSError| {
                if let Some(password) = string_value(result) {
                    let trimmed = password.trim();
                    if !trimmed.is_empty() && !EXITED.swap(true, Ordering::SeqCst) {
                        println!("{trimmed}");
                        std::process::exit(0);
                    }
                }
            });
            unsafe {
                self.ivars()
                    .webview
                    .evaluateJavaScript_completionHandler(&script, Some(&completion));
            }
        }
    }

    unsafe impl NSWindowDelegate for WebkitDelegate {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSNotification) {
            exit_once(1);
        }
    }
);

impl WebkitDelegate {
    fn new(webview: Retained<WKWebView>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DelegateIvars { webview });
        unsafe { msg_send![super(this), init] }
    }
}

fn string_value(result: *mut AnyObject) -> Option<String> {
    if result.is_null() {
        return None;
    }
    let object = unsafe { &*result };
    object.downcast_ref::<NSString>().map(|s| s.to_string())
}

pub fn webkit_main(url: &str, ignore_cert: bool) -> i32 {
    let Some(mtm) = MainThreadMarker::new() else {
        return 1;
    };

    // The WebKit portal always validates the certificate; fail loudly instead of silently ignoring
    // the flag and hanging until the timeout.
    if ignore_cert {
        eprintln!("--webkit-ignore-cert is not supported: the Mobile Access portal must present a trusted certificate");
        return 1;
    }

    // The child process only lives for the duration of the login; give up after the timeout.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(PASSWORD_TIMEOUT_SECS));
        exit_once(1);
    });

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
    // Ephemeral store so portal cookies and localStorage don't persist between logins.
    unsafe {
        configuration.setWebsiteDataStore(&WKWebsiteDataStore::nonPersistentDataStore(mtm));
    }
    let webview = unsafe { WKWebView::initWithFrame_configuration(WKWebView::alloc(mtm), frame, &configuration) };

    let delegate = WebkitDelegate::new(webview.clone(), mtm);
    unsafe {
        webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    }

    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str(&crate::tr!("label-mobile-access")));
    window.setContentView(Some(&webview));
    window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    window.center();
    window.makeKeyAndOrderFront(None);

    if let Some(nsurl) = NSURL::URLWithString(&NSString::from_str(url)) {
        let request = NSURLRequest::requestWithURL(&nsurl);
        unsafe {
            webview.loadRequest(&request);
        }
    }

    app.run();
    1
}
