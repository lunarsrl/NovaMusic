// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::{AppModel, Message};
use crate::config::AppTheme;
use crate::fl;
use cosmic::widget::settings::Section;
use cosmic::widget::text;
use cosmic::{theme, widget, Element};

impl AppModel {
    pub fn settings<'a>(&'a self) -> Element<'a, Message> {
        let cosmic::cosmic_theme::Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        let editable_settings: Section<Message> = cosmic::widget::settings::section();
        let current_settings: Section<Message> = cosmic::widget::settings::section();
        let player_settings: Section<Message> = cosmic::widget::settings::section();
        let ui_settings: Section<Message> = cosmic::widget::settings::section();

        let contain = widget::Container::new(
            widget::column::Column::with_children([
                cosmic::widget::toaster(&self.toasts, widget::horizontal_space()).into(),
                editable_settings
                    .title(fl!("MusicScanning"))
                    .add(widget::Row::with_children([
                        text::heading(fl!("MusicDirectory")).into(),
                        widget::horizontal_space().into(),
                        text::text(&self.config.scan_dir).into(),
                    ]))
                    .add(
                        widget::settings::item::builder(fl!("MusicDirectory"))
                            .description(fl!("firsttimebody"))
                            .control(match self.rescan_available {
                                true => widget::button::suggested(fl!("folderselect"))
                                    .on_press(Message::ChooseFolder),
                                false => widget::button::suggested(fl!("folderselect")),
                            }),
                    )
                    .add(
                        widget::settings::item::builder(fl!("FullRescan")).control(
                            match self.rescan_available && !self.config.scan_dir.is_empty() {
                                true => widget::button::text(fl!("Rescan"))
                                    .class(widget::button::ButtonClass::Destructive)
                                    .on_press(Message::RescanDir),
                                false => widget::button::text(fl!("Rescan"))
                                    .class(widget::button::ButtonClass::Destructive),
                            },
                        ),
                    )
                    .add(
                        widget::column::Column::with_children([
                            widget::row::Row::with_children([
                                text::heading(fl!("ScanProgress")).into(),
                                widget::horizontal_space().into(),
                                text::caption(format!(
                                    "{}%",
                                    (self.config.files_scanned as f32
                                        / self.config.num_files_found as f32
                                        * 100.0)
                                        .round()
                                ))
                                .into(),
                            ])
                            .into(),
                            widget::progress_bar(
                                0.0..=self.config.num_files_found as f32,
                                self.config.files_scanned as f32,
                            )
                            .height(space_s)
                            .into(),
                        ])
                        .spacing(space_xxs),
                    )
                    .into(),
                ui_settings
                    .title(fl!("UserInterface"))
                    .add(widget::settings::item::builder(fl! {"Theme"}).control(
                        cosmic::widget::dropdown(
                            vec!["Light", "Dark", "System"],
                            Some(match self.config.app_theme {
                                AppTheme::Light => 0,
                                AppTheme::Dark => 1,
                                AppTheme::System => 2,
                            }),
                            |a| {
                                Message::UpdateTheme(match a {
                                    0 => AppTheme::Light,
                                    1 => AppTheme::Dark,
                                    _ => AppTheme::System,
                                })
                            },
                        ),
                    ))
                    .add(
                        widget::settings::item::builder(fl!("GridItemSize")).control(
                            cosmic::widget::slider(1..=6, self.config.grid_item_size, |a| {
                                Message::GridSliderChange(a)
                            }),
                        ),
                    )
                    .add(
                        widget::settings::item::builder(fl! {"FooterToggle"}).control(
                            cosmic::widget::toggler(self.config.footer)
                                .on_toggle(|val| Message::ToggleFooter(val)),
                        ),
                    )
                    .into(),
                player_settings
                    .title(fl!("MusicPlayer"))
                    .add(
                        widget::settings::item::builder(fl!(
                            "AppVolume",
                            volume = self.config.volume.trunc()
                        ))
                        .control(cosmic::widget::slider(
                            0.0..=100.0,
                            self.config.volume,
                            |a| Message::VolumeSliderChange(a),
                        )),
                    )
                    .into(),
                current_settings
                    .title(fl!("CurrentScanResults"))
                    .add(widget::Row::with_children([
                        text::heading(fl!("FilesScanned")).into(),
                        widget::horizontal_space().into(),
                        text::text(format!(
                            "{}/{}",
                            self.config.num_files_found
                                - (self.config.num_files_found - self.config.files_scanned),
                            self.config.num_files_found
                        ))
                        .into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading(fl!("albums")).into(),
                        widget::horizontal_space().into(),
                        text::text(self.config.albums_found.to_string()).into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading(fl!("tracks")).into(),
                        widget::horizontal_space().into(),
                        text::text(self.config.tracks_found.to_string()).into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading(fl!("playlists")).into(),
                        widget::horizontal_space().into(),
                        text::text("None").into(),
                    ]))
                    .into(),
            ])
            .spacing(space_s),
        );
        contain.into()
    }
}
