use std::convert::TryInto;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, path::Path};

use registry::{key, value, Data, Hive, RegKey, Security};
use widestring::U16CString;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Key error")]
    Key(#[from] key::Error),

    #[error("Value error")]
    Value(#[from] value::Error),
}

pub(crate) fn open_key() -> Result<RegKey, Error> {
    Ok(Hive::LocalMachine.create(
        r"SOFTWARE\WinDivvun\Spellers".to_string(),
        Security::AllAccess,
    )?)
}

pub(crate) fn nuke_key() -> Result<(), Error> {
    let langs = Langs::new()?;
    deregister_langs(
        &langs
            .create
            .keys()
            .map(|x| x.to_string_lossy())
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}

const BASE_PROOF_TOOL_PATH: &str = r"SOFTWARE\Microsoft\Shared Tools\Proofing Tools\1.0\Override";
const PATH_CREATE: &str = "Create";
const PATH_DELETE: &str = "Delete";

const DIVVUNSPELL_MSO_32: &str = r"C:\Program Files\WinDivvun\divvunspell-mso32.dll";
const DIVVUNSPELL_MSO_64: &str = r"C:\Program Files\WinDivvun\divvunspell-mso64.dll";

fn add_create_key(base_path: &str, lang_id: &str, speller_path: &str) -> Result<(), Error> {
    // Check if value exists in Delete
    let full_delete_path = vec![base_path, PATH_DELETE, BASE_PROOF_TOOL_PATH, lang_id].join(r"\");

    match Hive::LocalMachine.delete(full_delete_path, true) {
        Err(key::Error::NotFound(_, _)) => { /* no problem. */ }
        Err(e) => return Err(e)?,
        _ => {}
    };

    // Now to create the Create record
    let full_create_path = vec![base_path, PATH_CREATE, BASE_PROOF_TOOL_PATH, lang_id].join(r"\");

    let regkey = Hive::LocalMachine.create(full_create_path, Security::AllAccess)?;
    regkey.set_value("LEX", &Data::String(speller_path.try_into().unwrap()))?;
    regkey.set_value("LEX64", &Data::String(speller_path.try_into().unwrap()))?;
    regkey.set_value("DLL", &Data::String(DIVVUNSPELL_MSO_32.try_into().unwrap()))?;
    regkey.set_value(
        "DLL64",
        &Data::String(DIVVUNSPELL_MSO_64.try_into().unwrap()),
    )?;

    Ok(())
}

fn add_delete_key(base_path: &str, lang_id: &str) -> Result<(), Error> {
    // Check if value exists in Create
    let full_create_path = vec![base_path, PATH_CREATE, BASE_PROOF_TOOL_PATH, lang_id].join(r"\");

    match Hive::LocalMachine.delete(full_create_path, true) {
        Err(key::Error::NotFound(_, _)) => { /* no problem. */ }
        Err(e) => return Err(e)?,
        _ => {}
    };

    // Now to create the Create record
    let full_delete_path = vec![base_path, PATH_DELETE, BASE_PROOF_TOOL_PATH, lang_id].join(r"\");

    let regkey = Hive::LocalMachine.create(full_delete_path, Security::AllAccess)?;
    regkey.set_value("LEX", &Data::String("".try_into().unwrap()))?;
    regkey.set_value("LEX64", &Data::String("".try_into().unwrap()))?;
    regkey.set_value("DLL", &Data::String("".try_into().unwrap()))?;
    regkey.set_value("DLL64", &Data::String("".try_into().unwrap()))?;

    Ok(())
}

pub(crate) fn register_langs(names: &[String], path: &Path) -> Result<(), Error> {
    let key = open_key()?;
    let display = path.to_string_lossy().to_string();
    let value: U16CString = display.clone().try_into().unwrap();

    for name in names {
        log::info!("Setting '{}' -> '{}'", name, &display);
        key.set_value(name, &Data::String(value.clone()))?;
    }

    log::info!(
        "Successfully set {} language tags for '{}'.",
        names.len(),
        path.display()
    );

    Ok(())
}

pub(crate) fn set_regkey_counter(base_path: &str) -> Result<(), Error> {
    let regkey = Hive::LocalMachine.create(base_path, Security::Write)?;
    // Our epoch starts at 2020-01-01.
    let our_epoch = UNIX_EPOCH + Duration::from_secs(1577836800);
    let ts: u32 = SystemTime::now()
        .duration_since(our_epoch)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap();
    regkey.set_value("Count", &Data::U32(ts))?;
    // No idea why this is needed, but nearly all other keys have it, so we do too.
    regkey.set_value("Order", &Data::U32(1))?;
    Ok(())
}

pub(crate) fn deregister_langs(names: &[String]) -> Result<(), Error> {
    let key = open_key()?;

    for name in names {
        log::info!("Setting '{}' -> <None>", name);
        key.set_value(name, &Data::None)?;
    }

    log::info!("Successfully unset {} language tags.", names.len());

    Ok(())
}

pub(crate) struct Langs {
    pub(crate) create: BTreeMap<U16CString, U16CString>,
    pub(crate) delete: Vec<U16CString>,
}

impl Langs {
    pub fn new() -> Result<Langs, Error> {
        let key = open_key()?;

        let mut create = BTreeMap::new();
        let mut delete = vec![];

        for (name, data) in key.values().flat_map(Result::ok).map(|x| x.into_inner()) {
            match data {
                Data::String(path) => {
                    create.insert(name, path);
                }
                Data::None => {
                    delete.push(name);
                }
                unhandled => log::warn!(
                    "Unhandled data for {}: {:?}",
                    &name.to_string_lossy(),
                    unhandled
                ),
            }
        }

        Ok(Langs { create, delete })
    }

    pub fn refresh(&self, base_path: &str) -> Result<(), Error> {
        for (lang_id, speller_path) in self.create.iter() {
            log::debug!("Adding create for {}", &lang_id.to_string_lossy());
            add_create_key(
                base_path,
                &lang_id.to_string_lossy(),
                &speller_path.to_string_lossy(),
            )?;
        }

        for lang_id in self.delete.iter() {
            log::debug!("Adding delete for {}", &lang_id.to_string_lossy());
            add_delete_key(base_path, &lang_id.to_string_lossy())?;
        }

        log::debug!("Updating count key");
        set_regkey_counter(base_path)?;

        Ok(())
    }
}
