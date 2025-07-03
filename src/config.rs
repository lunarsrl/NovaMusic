// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app;
use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use cosmic::Application;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub scan_dir: String,
    pub grid_item_size: u32,
    pub grid_item_spacing: u32,
    pub num_files_found: u32,
    pub files_scanned: u32
}

impl Config {
    pub fn load() -> (Option<cosmic_config::Config>, Config) {
        match cosmic_config::Config::new(app::AppModel::APP_ID, 1) {
            Ok(config_handler) => {
                let config = Config::get_entry(&config_handler).unwrap_or_else(|(errs, conf)| {
                    log::error!("Config failed to get entry: {:?}", errs);
                    conf
                });
                (Some(config_handler), config)
            }
            Err(e) => {
                log::error!("{}", e);
                (None, Config::default())
            }
        }
    }
}
