// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app;
use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use cosmic::Application;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AppTheme {
    Light,
    Dark,
    System,
}

impl AppTheme {
    pub fn theme(&self) -> cosmic::theme::Theme {
        match &self {
            AppTheme::Light => cosmic::theme::system_light(),
            AppTheme::Dark => cosmic::theme::system_dark(),
            AppTheme::System => cosmic::theme::system_preference(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub enum SortData {
    Time,
    String,
}

#[derive(Debug, Clone, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct Config {
    pub scan_dir: String,
    pub app_theme: AppTheme,
    pub grid_item_size: u32,
    pub num_files_found: u32,
    pub files_scanned: u32,
    pub tracks_found: u32,
    pub albums_found: u32,
    pub volume: f32,
    pub footer: bool,
    pub sort_order: SortOrder,
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
impl Default for Config {
    fn default() -> Self {
        Config {
            app_theme: AppTheme::System,
            scan_dir: "".to_string(),
            grid_item_size: 3,
            num_files_found: 0,
            files_scanned: 0,
            tracks_found: 0,
            albums_found: 0,
            volume: 100.0,
            footer: true,
            sort_order: SortOrder::Ascending,
        }
    }
}
