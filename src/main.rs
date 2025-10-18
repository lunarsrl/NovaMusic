// SPDX-License-Identifier: GPL-2.0-or-later

use crate::database::create_database;
use crate::log::setup_logger;
use ::log::info;
use std::fmt::Display;
use std::fs;
use strum_macros::EnumString;
use symphonia::core::meta::StandardTagKey;

mod app;
mod config;
mod database;
mod i18n;
mod log;

fn main() -> cosmic::iced::Result {
    //start logging
    let logger = setup_logger();
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
