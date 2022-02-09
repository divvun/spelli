use crate::refresh::get_candidate_regkeys;
use std::{path::PathBuf, process::Command};

pub fn nuke() {
    if let Some(unopkg) = find_unopkg() {
        let result = Command::new(&unopkg)
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
    } else {
        log::info!("Unable to find unopkg, not uninstalling from libreoffice");
    }

    let install_path = get_speller_install_directory();
    if install_path.exists() {
        if let Err(e) = std::fs::remove_dir_all(&install_path) {
            log::info!("Unable to remove {:?}: {}", install_path, e)
        }
    } else {
        log::info!("Unable to find oxt on disk, not removing");
    }
}

pub fn find_unopkg() -> Option<PathBuf> {
    let libreoffice_install = get_candidate_regkeys()
        .iter()
        .find_map(|candidate| candidate.validate_libreoffice());

    let libreoffice_install = match libreoffice_install {
        None => {
            log::info!("Did not find any LibreOffice installation, not installing LibreOffice spellchecker extension");
            return None;
        }
        Some(install) => install,
    };

    let unopkg_path = libreoffice_install
        .install_path
        .join("program")
        .join("unopkg.com");
    log::debug!("Checking if unopkg exists at: {:?}", &unopkg_path);

    if !unopkg_path.exists() {
        log::error!(
            "Couldn't find unopkg at {:?}. Is the installation corrupt?",
            &unopkg_path
        );
        return None;
    }

    Some(unopkg_path)
}

pub fn get_speller_install_directory() -> PathBuf {
    let program_files_path = windirs::known_folder_path(windirs::FolderId::ProgramFiles).unwrap();
    program_files_path.join("DivvunSpell LibreOffice")
}
