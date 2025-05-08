use std::borrow::Cow;
use std::sync::{LazyLock, RwLock};

use cached::proc_macro::cached;
use fluent_templates::{LanguageIdentifier, Loader, static_loader};

pub use fluent_templates;
use fluent_templates::fluent_bundle::FluentValue;

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
        i18n::translate($message_id)
    };

    ($message_id:literal, $($key:ident = $value:expr),*) => {
        {
                i18n::translate_with_args(
                    stringify!($message_id),
                    [$((std::borrow::Cow::Borrowed(stringify!($key)), $value.to_string().into()))*])
        }
    };
}

pub fn translate(key: &str) -> String {
    LOCALES.lookup(&get_locale(), key)
}

pub fn translate_with_args<I>(key: &str, args: I) -> String
where
    I: IntoIterator<Item = (Cow<'static, str>, FluentValue<'static>)>,
{
    let args = args.into_iter().collect::<std::collections::HashMap<_, _>>();
    LOCALES.lookup_with_args(&get_locale(), key, &args)
}

#[cached]
pub fn get_user_locale() -> LanguageIdentifier {
    let lang = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    lang.parse().or_else(|_| "en-US".parse()).unwrap()
}

pub fn set_locale(lang: Option<LanguageIdentifier>) {
    *APP_LOCALE.write().unwrap() = lang;
}

pub fn get_locale() -> LanguageIdentifier {
    APP_LOCALE.read().unwrap().clone().unwrap_or_else(get_user_locale)
}

pub fn get_locales() -> Vec<LanguageIdentifier> {
    LOCALES.locales().cloned().collect()
}
