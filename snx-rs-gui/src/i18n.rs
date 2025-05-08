use std::sync::RwLock;

use i18n_embed::{
    DesktopLanguageRequester,
    fluent::{FluentLanguageLoader, fluent_language_loader},
    unic_langid::LanguageIdentifier,
};
use once_cell::sync::Lazy;
use tracing::error;

use crate::assets;

pub struct LoaderHolder {
    pub loader: RwLock<FluentLanguageLoader>,
}

pub static HOLDER: Lazy<LoaderHolder> = Lazy::new(|| LoaderHolder {
    loader: RwLock::new(new_language_loader(None::<&str>)),
});

fn new_language_loader<S>(fallback_locale: Option<S>) -> FluentLanguageLoader
where
    S: AsRef<str>,
{
    let languages = match fallback_locale {
        Some(loc) => match loc.as_ref().parse::<LanguageIdentifier>() {
            Ok(lang) => vec![lang],
            Err(e) => {
                error!("{}", e);
                DesktopLanguageRequester::requested_languages()
            }
        },
        None => DesktopLanguageRequester::requested_languages(),
    };
    let loader = fluent_language_loader!();
    let _ = i18n_embed::select(&loader, &assets::Localizations, &languages);
    loader.set_use_isolating(false);
    loader
}

#[macro_export]
macro_rules! tr {
    ($message_id:literal) => {
        i18n_embed_fl::fl!($crate::i18n::HOLDER.loader.read().unwrap(), $message_id)
    };

    ($message_id:literal, $($key:ident = $value:expr),*) => {
        i18n_embed_fl::fl!($crate::i18n::HOLDER.loader.read().unwrap(), $message_id, $($key = $value), *)
    };
}
