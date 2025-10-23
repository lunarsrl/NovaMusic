// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::iced::alignment::Horizontal;
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
        let grid_settings: Section<Message> = cosmic::widget::settings::section();

        let contain = widget::Container::new(
            widget::column::Column::with_children([
                cosmic::widget::toaster(&self.toasts, widget::horizontal_space()).into(),
                current_settings
                    .title(fl!("CurrentScanResults"))
                    .add(widget::Row::with_children([
                        text::heading(fl!("MusicDirectory")).into(),
                        widget::horizontal_space().into(),
                        text::text(&self.config.scan_dir).into(),
                    ]))
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
                editable_settings
                    .title(fl!("MusicScanning"))
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
                            widget::column::Column::with_children([
                                text::heading(fl!("ScanProgress")).into(),
                                text::caption(format!(
                                    "{}%",
                                    (self.config.files_scanned as f32
                                        / self.config.num_files_found as f32
                                        * 100.0)
                                        .round()
                                ))
                                .align_x(Horizontal::Right)
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
                grid_settings
                    .title(fl!("UserInterface"))
                    .add(
                        widget::settings::item::builder(fl!("GridItemSize")).control(
                            cosmic::widget::slider(1..=6, self.config.grid_item_size, |a| {
                                Message::GridSliderChange(a)
                            }),
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
            ])
            .spacing(space_s),
        );
        contain.into()
    }
}
