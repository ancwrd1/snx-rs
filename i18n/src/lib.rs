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

#[macro_export]
macro_rules! tr {
    ($message_id:literal) => {
        i18n::fluent_templates::Loader::lookup(&*i18n::LOCALES, &i18n::get_user_locale(), $message_id)
    };

    ($message_id:literal, $($key:ident = $value:expr),*) => {
        {
            let mut args = std::collections::HashMap::new();
            $(args.insert(std::borrow::Cow::Borrowed(stringify!($key)), $value.to_string().into());)*
            i18n::fluent_templates::Loader::lookup_with_args(&*i18n::LOCALES, &i18n::get_user_locale(), $message_id, &args)
        }
    };
}

pub fn translate(key: &str) -> String {
    LOCALES.lookup(&get_user_locale(), key)
}

#[cached]
pub fn get_user_locale() -> LanguageIdentifier {
    let lang = std::env::var("SNXRS_LOCALE")
        .ok()
        .or_else(sys_locale::get_locale)
        .unwrap_or_else(|| "en-US".to_string());
    LanguageIdentifier::from_bytes(lang.as_bytes())
        .or_else(|_| LanguageIdentifier::from_bytes("en-US".as_bytes()))
        .unwrap()
}
