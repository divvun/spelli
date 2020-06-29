use crate::reg;
use registry::{Data, Hive, RegKey, Security};

pub(crate) fn refresh() {
    // Iterate relevant registry key for all lang-id -> zhfst path value pairs
    log::info!("Detecting MS Office installations...");

    let offices = detect_ms_office();
    if offices.is_empty() {
        log::warn!("No Office installations detected; aborting.");
        return;
    }

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

    log::info!("Refresh completed.");
}

const KEY_UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const KEY_UNINSTALL_WOW64: &str =
    r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

#[derive(Debug)]
struct Office {
    variant: InstallMethod,
    major_version: u32,
}

impl Office {
    fn user_settings_paths_wow64(&self) -> Option<&[&str]> {
        log::debug!("Getting user settings path for 64-bit Windows installation...");

        match (&self.variant, self.major_version) {
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

fn parse_office_key(regkey: &RegKey) -> Option<Office> {
    let publisher = regkey.value("Publisher").ok()?;
    let display_name = regkey.value("DisplayName").ok()?;
    let display_version = regkey.value("DisplayVersion").ok()?;
    let install_location = regkey.value("InstallLocation").ok()?;
    let click_to_run_component = regkey.value("ClickToRunComponent");

    match publisher {
        Data::String(s) if s.to_string_lossy() == "Microsoft Corporation" => {}
        _ => return None,
    };

    match display_name {
        Data::String(s) if s.to_string_lossy().starts_with("Microsoft Office") => {}
        _ => return None,
    }

    match install_location {
        Data::String(s) if s.to_string_lossy().ends_with("Microsoft Office") => {}
        _ => return None,
    }

    let major_version: u32 = match display_version {
        Data::String(s) => s
            .to_string_lossy()
            .split(".")
            .next()
            .and_then(|x| x.parse::<u32>().ok())?,
        _ => return None,
    };

    let is_click_to_run = click_to_run_component.is_ok();

    Some(Office {
        variant: if is_click_to_run {
            InstallMethod::Click2Run
        } else {
            InstallMethod::Msi
        },
        major_version,
    })
}

fn detect_ms_office() -> Vec<Office> {
    log::debug!("Opening primary uninstall key");
    let regkey = Hive::LocalMachine
        .open(KEY_UNINSTALL, Security::Read)
        .unwrap();
    let regkey_wow64 = Hive::LocalMachine
        .open(KEY_UNINSTALL_WOW64, Security::Read)
        .unwrap();

    let iter = regkey
        .keys()
        .flat_map(Result::ok)
        .chain(regkey_wow64.keys().flat_map(Result::ok));

    let office_installs = iter
        .flat_map(|keyref| {
            let subkey = keyref.open(Security::Read).ok()?;
            parse_office_key(&subkey)
        })
        .collect::<Vec<_>>();

    for office in office_installs.iter() {
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