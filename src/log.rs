// SPDX-License-Identifier: GPL-2.0-or-later

use colored::Colorize;
use log::Level;
use std::time::SystemTime;

pub fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                match record.level() {
                    Level::Error => record.level().to_string().red(),
                    Level::Warn => {
                        record.level().to_string().yellow()
                    }
                    Level::Info => {
                        record.level().to_string().blue()
                    }
                    Level::Debug => {
                        record.level().to_string().green()
                    }
                    Level::Trace => {
                        record.level().to_string().purple()
                    }
                },
                record.target(),
                message
            ));
        })
        .level_for("wgpu_core::device::resource", log::LevelFilter::Error)
        .level_for("tracing::span", log::LevelFilter::Error)
        .level_for("symphonia_bundle_mp3::demuxer", log::LevelFilter::Error)
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}
