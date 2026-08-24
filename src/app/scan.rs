// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::ReEnterNavReason;
use crate::database::create_database_entry;
use cosmic::Application;
use rusqlite::Transaction;
use std::fs;
use std::path::{Path, PathBuf};
use symphonia::default::get_probe;

pub fn scan_directory<'a>(path: PathBuf, tx: &Transaction) {
    let mut index = 0;
    read_dir(path, tx, &mut index);
}

fn read_dir<'a>(path: PathBuf, tx: &Transaction, index: &mut u32) {
    if let Ok(dir) = path.read_dir() {
        for entry in dir {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Ok(entry) = entry.metadata() {
                    if entry.is_dir() {
                        read_dir(path, tx, index)
                    } else {
                        handle_file(tx, path)
                    }
                }
            }
        }
    } else {
        println!("Error at path: {}", path.to_string_lossy().to_string())
    }
}

fn handle_file(tx: &rusqlite::Transaction, path: PathBuf) {
    let file = fs::File::open(&path).unwrap();
    let probe = get_probe();
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    if let Ok(mut reader) = probe.probe(
        &Default::default(),
        mss,
        Default::default(),
        Default::default(),
    ) {
        let mut mdat = reader.metadata();
        if let Some(tags) = mdat.skip_to_latest() {
            let tags = tags
                .media
                .tags
                .iter()
                .filter(|a| a.has_std_tag())
                .map(|a| a.clone())
                .collect();
            create_database_entry(tags, &path, tx);
        }
    } else {
        if path.with_extension("m3u") == path || path.with_extension("m3u8") == path {
            let mut dir = PathBuf::new();

            if dirs::data_local_dir()
                .unwrap()
                .join(crate::app::AppModel::APP_ID)
                .join("Playlists")
                .exists()
            {
                dir = dirs::data_local_dir()
                    .unwrap()
                    .join(crate::app::AppModel::APP_ID)
                    .join("Playlists");
            } else {
                match std::fs::create_dir(
                    dirs::data_local_dir()
                        .unwrap()
                        .join(crate::app::AppModel::APP_ID)
                        .join("Playlists"),
                ) {
                    Ok(_) => {
                        dir = dirs::data_local_dir()
                            .unwrap()
                            .join(crate::app::AppModel::APP_ID)
                            .join("Playlists");
                    }
                    Err(err) => {}
                }
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            fs::copy(path, dir.as_path().join(name)).unwrap();
        } else {
            log::info!(
                "ERROR: Probe failure \nErred Path: {}",
                path.to_str().unwrap().to_string()
            );
        }
    }
}
