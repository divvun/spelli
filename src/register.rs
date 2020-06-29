use crate::RegisterArgs;

use std::{collections::HashSet, convert::Infallible};
use unic_langid::{
    subtags::{Region, Script},
    LanguageIdentifier,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Invalid language tag")]
    InvalidLanguageTag(#[from] unic_langid::LanguageIdentifierError),

    #[error("No default script found for language tag")]
    NoDefaultScript,

    #[error("Could not update registry")]
    Registry(#[from] crate::reg::Error),
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub(crate) fn derive_lang_id_keys(mut lang_id: LanguageIdentifier) -> Result<Vec<String>, Error> {
    let mut keys = HashSet::new();

    let key = lang_id.to_string();
    log::debug!("Adding '{}' to keys", &key);
    keys.insert(key);

    let lcid = iso639::lcid::get(
        lang_id.language.as_str(),
        lang_id.script.as_ref().map(|x| x.as_str()),
        lang_id.region.as_ref().map(|x| x.as_str()),
    );

    match lcid {
        Some(v) => {
            log::info!("Tag has LCID: {:08x}", v.lcid);
            let key = lang_id.to_string();

            log::debug!("Adding '{}' to keys", &key);
            keys.insert(key);
        }
        None => {
            log::info!("No LCID for given tag.");

            match lang_id.script.as_ref().map(|x| x.as_str()) {
                Some(v) => {
                    log::info!("Using provided script: {}", v);
                }
                None => {
                    let script = iso639::script::get(lang_id.language.as_str())
                        .ok_or_else(|| Error::NoDefaultScript)?
                        .script;
                    log::info!("Using derived default script: {}", script);
                    lang_id.script = Script::from_bytes(script.as_bytes()).ok();
                }
            };

            match lang_id.region.as_ref().map(|x| x.as_str()) {
                Some(v) => {
                    log::info!("Using provided region: {}", v);
                }
                None => {
                    log::info!("Using fallback region '001'");
                    let mut lang_id = lang_id.clone();
                    lang_id.region = Some(Region::from_bytes(b"001").unwrap());
                    let key = lang_id.to_string();

                    log::debug!("Adding '{}' to keys", &key);
                    keys.insert(key);
                }
            };

            let key = lang_id.to_string();
            log::debug!("Adding '{}' to keys", &key);
            keys.insert(key);
        }
    }

    let mut keys = keys.into_iter().collect::<Vec<_>>();
    keys.sort();
    Ok(keys)
}

pub(crate) fn register(args: RegisterArgs) -> Result<(), Error> {
    let lang_id: LanguageIdentifier = args.tag.parse()?;
    log::info!("Registering speller for '{}'...", &lang_id);

    let keys = derive_lang_id_keys(lang_id)?;
    crate::reg::register_langs(&keys, &*args.path)?;

    crate::refresh::refresh();

    log::info!("Registration complete!");

    Ok(())
}
