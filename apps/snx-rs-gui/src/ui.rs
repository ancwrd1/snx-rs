use std::{cell::RefCell, collections::HashMap, rc::Rc};

use slint::{ComponentHandle, Global};
use snxcore::model::params::ColorTheme;

use crate::profiles::ConnectionProfilesStore;

pub mod about;
pub mod prompt;
pub mod settings;
pub mod status;

slint::include_modules!();

include!(concat!(env!("OUT_DIR"), "/tr_setters.rs"));

thread_local! {
    static OPEN_WINDOWS: RefCell<HashMap<&'static str, Rc<dyn WindowController>>> =
        RefCell::new(HashMap::new());
}

pub fn open_window<F>(name: &'static str, factory: F)
where
    F: FnOnce() -> anyhow::Result<Rc<dyn WindowController>> + Send + 'static,
{
    let _ = slint::invoke_from_event_loop(|| {
        if !OPEN_WINDOWS.with(|slot| slot.borrow().contains_key(name))
            && let Ok(controller) = factory()
            && controller.present().is_ok()
        {
            store_window(controller.name(), controller);
        }
    });
}

pub fn run_from_event_loop<F, R>(f: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    slint::Timer::single_shot(std::time::Duration::ZERO, || {
        tokio::spawn(f);
    });
}

pub fn update_windows() {
    for controller in OPEN_WINDOWS.with(|slot| slot.borrow().values().cloned().collect::<Vec<_>>()) {
        controller.update();
    }
}

fn store_window(name: &'static str, controller: Rc<dyn WindowController>) {
    let this = controller.clone();
    OPEN_WINDOWS.with(move |slot| slot.borrow_mut().insert(name, this));
}

fn close_window(name: &'static str) {
    OPEN_WINDOWS.with(move |slot| slot.borrow_mut().remove(name));
}

pub trait WindowController {
    fn present(&self) -> anyhow::Result<()>;

    fn name(&self) -> &'static str;

    fn update(&self);
}

struct WindowScope<C: ComponentHandle> {
    pub window: C,
}

impl<'a, C> WindowScope<C>
where
    C: ComponentHandle,
    Tr<'a>: Global<'a, C>,
    Palette<'a>: Global<'a, C>,
{
    fn new(window: C) -> Rc<Self> {
        Rc::new(Self { window })
    }

    fn set_globals(&'a self) {
        apply_translations(&self.window.global::<Tr>());
        self.set_color_theme();
    }

    fn set_color_theme(&'a self) {
        let scheme = match ConnectionProfilesStore::instance().get_default().color_theme {
            ColorTheme::Light => slint::language::ColorScheme::Light,
            ColorTheme::Dark => slint::language::ColorScheme::Dark,
            ColorTheme::AutoDetect => return,
        };

        let palette = self.window.global::<Palette>();
        palette.set_color_scheme(scheme);
    }

    fn weak(self: &Rc<Self>) -> std::rc::Weak<Self> {
        Rc::downgrade(self)
    }
}

impl<C: ComponentHandle> Drop for WindowScope<C> {
    fn drop(&mut self) {
        let _ = self.window.hide();
    }
}
