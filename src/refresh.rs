use crate::{libreoffice, reg, register};
use registry::{Data, Hive, RegKey, Security};
use serde::Deserialize;
use std::{
    collections::BTreeMap, fmt::Display, fs::File, io::Read, path::PathBuf, process::Command,
};
use unic_langid::LanguageIdentifier;

const OXT_DATA: &[u8] = include_bytes!("../divvunspell-libreoffice.oxt");

#[derive(Debug, Clone, Deserialize)]
struct SpellerToml {
    spellers: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("A registry error occurred")]
    Registry(#[from] reg::Error),

    #[error("An IO error occurred")]
    Io(#[from] std::io::Error),

    #[error("Invalid language tag")]
    InvalidLanguageTag(#[from] unic_langid::LanguageIdentifierError),
}

pub(crate) fn refresh() -> Result<(), Error> {
    log::info!("Beginning refresh process");

    // Try to read the spellers directory before we blindly delete everything
    let speller_dirs = std::fs::read_dir(reg::SPELLER_DIR)?;
    let speller_tomls: Vec<(PathBuf, SpellerToml)> = speller_dirs
        .into_iter()
        .filter_map(|x| match x {
            Ok(v) if v.metadata().map(|m| m.is_dir()).unwrap_or(false) => Some(v.path()),
            _ => None,
        })
        .filter_map(|path| {
            let p = path.join("speller.toml");
            match File::open(&p) {
                Ok(v) => Some((path, v)),
                Err(e) => {
                    log::error!("Error loading speller.toml at path: {}", path.display());
                    log::error!("{:?}", e);
                    None
                }
            }
        })
        .filter_map(|(path, mut file)| {
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error reading `{}`", path.display());
                    log::error!("{:?}", e);
                    return None;
                }
            }
            match toml::from_str(&s) {
                Ok(x) => Some((path, x)),
                Err(e) => {
                    log::error!("Error parsing `{}`", path.display());
                    log::error!("{:?}", e);
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    // Remove all currently added languages
    reg::nuke_key()?;

    // Add languages that exist with a valid toml file
    for (toml_path, speller_toml) in speller_tomls {
        log::info!("Reading {}...", toml_path.display());

        for (tag, path) in speller_toml.spellers.iter() {
            let lang_id: LanguageIdentifier = tag.parse()?;
            log::info!("Registering speller for '{}'...", &lang_id);

            let keys = match register::derive_lang_id_keys(lang_id) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Error deriving language keys for `{}`", tag);
                    log::error!("{:?}", e);
                    continue;
                }
            };

            crate::reg::register_langs(&keys, &toml_path.join(path))?;
        }
    }

    // Iterate relevant registry key for all lang-id -> zhfst path value pairs
    log::info!("Detecting MS Office installations...");
    let _unused = detect_ms_office();

    let offices = Office::all_supported();
    // if offices.is_empty() {
    //     log::warn!("No Office installations detected; aborting.");
    //     return Ok(());
    // }

    let langs = reg::Langs::new().unwrap();
    for paths in offices.iter().flat_map(|x| x.user_settings_paths()) {
        for path in paths {
            match langs.refresh(path) {
                Ok(_) => {
                    log::info!("Refreshed reg keys for {}", &path);
                }
                Err(err) => {
                    log::error!("Error for {}: {:?}", &path, err);
                }
            }
        }
    }

    refresh_libreoffice_spellchecker();

    log::info!("Refresh completed.");
    Ok(())
}

const KEY_UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

pub struct LibreOffice {
    pub install_path: PathBuf,
}

#[derive(Debug)]
struct Office {
    variant: InstallMethod,
    major_version: u32,
}

impl Office {
    fn all_supported() -> Vec<Office> {
        let mut out = vec![];

        for i in 10..=16 {
            out.push(Office {
                variant: InstallMethod::Msi,
                major_version: i,
            });
        }
        for i in 15..=16 {
            out.push(Office {
                variant: InstallMethod::Click2Run,
                major_version: i,
            });
        }

        out
    }

    fn user_settings_paths_wow64(&self) -> Option<&[&str]> {
        log::debug!("Getting user settings path for 64-bit Windows installation...");

        match (&self.variant, self.major_version) {
            (InstallMethod::Msi, 14) => Some(&[
                r"SOFTWARE\Microsoft\Office\14.0\User Settings\WinDivvun",
                r"SOFTWARE\Wow6432Node\Microsoft\Office\14.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Click2Run, 14) => Some(&[
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Microsoft\Office\14.0\User Settings\WinDivvun",
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Wow6432Node\Microsoft\Office\14.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Msi, 15) => Some(&[
                r"SOFTWARE\Microsoft\Office\15.0\User Settings\WinDivvun",
                r"SOFTWARE\Wow6432Node\Microsoft\Office\15.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Click2Run, 15) => Some(&[
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Microsoft\Office\15.0\User Settings\WinDivvun",
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Wow6432Node\Microsoft\Office\15.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Msi, 16) => Some(&[
                r"SOFTWARE\Microsoft\Office\16.0\User Settings\WinDivvun",
                r"SOFTWARE\Wow6432Node\Microsoft\Office\16.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Click2Run, 16) => Some(&[
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Microsoft\Office\16.0\User Settings\WinDivvun",
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Wow6432Node\Microsoft\Office\16.0\User Settings\WinDivvun",
            ]),
            _ => {
                log::error!(
                    "Unhandled Office variant! {:?} {:?}",
                    self.variant,
                    self.major_version
                );
                None
            }
        }
    }

    fn user_settings_paths(&self) -> Option<&[&str]> {
        // Detect if the OS has WOW64 support the worst possible way
        if Hive::LocalMachine
            .open(r"SOFTWARE\Wow6432Node", Security::Read)
            .is_ok()
        {
            return self.user_settings_paths_wow64();
        }

        log::debug!("Getting user settings path for 32-bit (or 64-bit missing WOW64) Windows installation...");

        match (&self.variant, self.major_version) {
            (InstallMethod::Msi, 14) => {
                Some(&[r"SOFTWARE\Microsoft\Office\14.0\User Settings\WinDivvun"])
            }
            (InstallMethod::Click2Run, 14) => Some(&[
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Microsoft\Office\14.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Msi, 15) => {
                Some(&[r"SOFTWARE\Microsoft\Office\15.0\User Settings\WinDivvun"])
            }
            (InstallMethod::Click2Run, 15) => Some(&[
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Microsoft\Office\15.0\User Settings\WinDivvun",
            ]),
            (InstallMethod::Msi, 16) => {
                Some(&[r"SOFTWARE\Microsoft\Office\16.0\User Settings\WinDivvun"])
            }
            (InstallMethod::Click2Run, 16) => Some(&[
                r"SOFTWARE\Microsoft\Office\ClickToRun\REGISTRY\MACHINE\Software\Microsoft\Office\16.0\User Settings\WinDivvun",
            ]),
            _ => {
                log::error!(
                    "Unhandled Office variant! {:?} {:?}",
                    self.variant,
                    self.major_version
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateRegKey {
    publisher: Option<Data>,
    display_name: Option<Data>,
    display_version: Option<Data>,
    install_location: Option<Data>,
    click_to_run_component: Option<Data>,
}

impl Display for CandidateRegKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Publisher: {:?}",
            self.publisher.as_ref().map(|x| x.to_string())
        )?;
        writeln!(
            f,
            "DisplayName: {:?}",
            &self.display_name.as_ref().map(|x| x.to_string())
        )?;
        writeln!(
            f,
            "DisplayVersion: {:?}",
            &self.display_version.as_ref().map(|x| x.to_string())
        )?;
        writeln!(
            f,
            "InstallLocation: {:?}",
            &self.install_location.as_ref().map(|x| x.to_string())
        )?;
        writeln!(f, "ClickToRunComponent: {:?}", self.click_to_run_component)?;
        Ok(())
    }
}

impl From<&RegKey> for CandidateRegKey {
    fn from(regkey: &RegKey) -> Self {
        let publisher = regkey.value("Publisher").ok();
        let display_name = regkey.value("DisplayName").ok();
        let display_version = regkey.value("DisplayVersion").ok();
        let install_location = regkey.value("InstallLocation").ok();
        let click_to_run_component = regkey.value("ClickToRunComponent").ok();

        Self {
            publisher,
            display_name,
            display_version,
            install_location,
            click_to_run_component,
        }
    }
}

impl CandidateRegKey {
    fn validate_office(&self) -> Option<Office> {
        if self.publisher.as_ref()?.to_string() != "Microsoft Corporation" {
            return None;
        }

        if !self
            .display_name
            .as_ref()?
            .to_string()
            .starts_with("Microsoft Office")
        {
            return None;
        }

        let major_version: u32 = match self.display_version.as_ref() {
            Some(Data::String(s)) => s
                .to_string_lossy()
                .split(".")
                .next()
                .and_then(|x| x.parse::<u32>().ok())?,
            _ => return None,
        };

        let is_click_to_run = self.click_to_run_component.is_some();

        Some(Office {
            variant: if is_click_to_run {
                InstallMethod::Click2Run
            } else {
                InstallMethod::Msi
            },
            major_version,
        })
    }

    pub fn validate_libreoffice(&self) -> Option<LibreOffice> {
        if !self
            .display_name
            .as_ref()?
            .to_string()
            .starts_with("LibreOffice")
        {
            return None;
        }

        self.install_location
            .as_ref()
            .map(|install_path| LibreOffice {
                install_path: PathBuf::from(install_path.to_string()),
            })
    }
}

pub(crate) fn get_candidate_regkeys() -> Vec<CandidateRegKey> {
    let regkey = Hive::LocalMachine
        .open(KEY_UNINSTALL, Security::Read | Security::Wow6464Key)
        .unwrap();
    let regkey_wow64 = Hive::LocalMachine
        .open(KEY_UNINSTALL, Security::Read | Security::Wow6432Key)
        .unwrap();

    let iter = regkey
        .keys()
        .flat_map(Result::ok)
        .chain(regkey_wow64.keys().flat_map(Result::ok));

    iter.filter_map(|keyref| {
        let subkey = match keyref.open(Security::Read) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{:?}", e);
                return None;
            }
        };
        log::trace!("Parsing: {}", regkey);
        Some(CandidateRegKey::from(&subkey))
    })
    .collect::<Vec<_>>()
}

fn detect_ms_office() -> Vec<Office> {
    let office_installs = get_candidate_regkeys()
        .iter()
        .filter_map(|candidate| candidate.validate_office())
        .collect::<Vec<_>>();

    log::trace!("Opening primary uninstall key");
    for office in &office_installs {
        log::info!(
            "Found Office {} {:?}!",
            &office.major_version,
            &office.variant
        );
    }

    office_installs
}

#[derive(Debug)]
enum InstallMethod {
    Click2Run,
    Msi,
}

fn refresh_libreoffice_spellchecker() {
    let unopkg_path = libreoffice::find_unopkg();
    if unopkg_path.is_none() {
        log::error!("Couldn't find unopkg, aborting LibreOffice spellechecker installation");
        return;
    }
    let unopkg_path = unopkg_path.unwrap();

    let install_path = libreoffice::get_speller_install_directory();
    if let Err(e) = std::fs::create_dir_all(&install_path) {
        log::error!(
            "Failed to create install directory for LibreOffice speller: {}",
            e
        );
        return;
    }

    let oxt_path = install_path.join("divvunspell.oxt");
    if let Err(e) = std::fs::write(&oxt_path, OXT_DATA) {
        log::error!("Failed to write divvunspell.oxt: {}", e);
        return;
    }

    log::info!("Trying to remove previous installation if it exists");
    let result = Command::new(&unopkg_path)
        .args(&["remove", "--shared", "no.divvun.DivvunSpell"])
        .output();

    match result {
        Err(e) => {
            log::error!("Failed to start unokpg process: {}", e);
            return;
        }
        Ok(v) => {
            // We don't care if it fails here, it means that it wasn't installed in the first place
            log::debug!("Unopkg remove exited with status: {}", v.status)
        }
    }

    let result = Command::new(unopkg_path)
        .args(&["add", "--shared", oxt_path.to_str().unwrap()])
        .output();

    match result {
        Err(e) => {
            log::error!("Failed to start unokpg process: {}", e);
            return;
        }
        Ok(v) => {
            // We don't care if it fails here, it means that it wasn't installed in the first place
            log::debug!("Unopkg install exited with status: {}", v.status);
            if !v.status.success() {
                log::error!("Failed to install the libreoffice extension");
                log::error!("stdout: {}", &String::from_utf8_lossy(&v.stdout));
                log::error!("stderr: {}", &String::from_utf8_lossy(&v.stderr));
                return;
            }
        }
    }
}
