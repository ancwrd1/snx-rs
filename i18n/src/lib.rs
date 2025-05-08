use std::sync::{LazyLock, RwLock};

use cached::proc_macro::cached;
use fluent_templates::{LanguageIdentifier, Loader, static_loader};

pub use fluent_templates;

static_loader! {
    pub static LOCALES = {
        locales: "./assets",
        fallback_language: "en-US",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

static APP_LOCALE: LazyLock<RwLock<Option<LanguageIdentifier>>> = LazyLock::new(|| RwLock::new(None));

#[macro_export]
macro_rules! tr {
    ($message_id:literal) => {
        i18n::fluent_templates::Loader::lookup(&*i18n::LOCALES, &i18n::get_locale(), $message_id)
    };

    ($message_id:literal, $($key:ident = $value:expr),*) => {
        {
            let mut args = std::collections::HashMap::new();
            $(args.insert(std::borrow::Cow::Borrowed(stringify!($key)), $value.to_string().into());)*
            i18n::fluent_templates::Loader::lookup_with_args(&*i18n::LOCALES, &i18n::get_locale(), $message_id, &args)
        }
    };
}

pub fn translate(key: &str) -> String {
    LOCALES.lookup(&get_user_locale(), key)
}

#[cached]
pub fn get_user_locale() -> LanguageIdentifier {
    let lang = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    lang.parse().or_else(|_| "en-US".parse()).unwrap()
}

pub fn set_locale(lang: LanguageIdentifier) {
    APP_LOCALE.write().unwrap().replace(lang);
}

pub fn get_locale() -> LanguageIdentifier {
    APP_LOCALE.read().unwrap().clone().unwrap_or_else(get_user_locale)
}

pub fn get_locales() -> Vec<LanguageIdentifier> {
    LOCALES.locales().cloned().collect()
}
