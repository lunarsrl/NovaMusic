use crate::app;
use crate::app::{AppModel, Message};
use cosmic::widget::text::heading;
use cosmic::widget::{container, icon, text, ProgressBar};
use cosmic::{theme, widget, Element};
use i18n_embed_fl::fl;
use std::borrow::Cow;
use std::fmt::format;
use std::ops::RangeInclusive;
use cosmic::iced::Alignment;
use cosmic::iced::alignment::Horizontal;
use cosmic::widget::settings::Section;

impl AppModel {
    pub fn settings(&self) -> Element<app::Message> {
        let cosmic::cosmic_theme::Spacing {
            space_xxs,
            space_s,
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        let editable_settings: Section<Message> = cosmic::widget::settings::section();
        let current_settings: Section<Message> = cosmic::widget::settings::section();
        let player_settings: Section<Message> = cosmic::widget::settings::section();
        let grid_settings: Section<Message> = cosmic::widget::settings::section();


        let contain = widget::Container::new(
            widget::column::Column::with_children([
                current_settings
                    .title("Current Scan Results")
                    .add(widget::Row::with_children([
                        text::heading("Music Directory:").into(),
                        widget::horizontal_space().into(),
                        text::text(&self.config.scan_dir).into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading("Files Scanned:").into(),
                        widget::horizontal_space().into(),
                        text::text(format!(
                            "{}/{}",
                            (self.config.num_files_found
                                - (self.config.num_files_found - self.config.files_scanned)),
                            self.config.num_files_found
                        ))
                        .into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading("Albums Found:").into(),
                        widget::horizontal_space().into(),
                        text::text(self.config.albums_found.to_string()).into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading("Tracks Found:").into(),
                        widget::horizontal_space().into(),
                        text::text(self.config.tracks_found.to_string()).into(),
                    ]))
                    .add(widget::Row::with_children([
                        text::heading("Playlists Found:").into(),
                        widget::horizontal_space().into(),
                        text::text("None").into(),
                    ]))
                    .into(),
                editable_settings
                    .title("Music Scanning")
                    .add(
                        widget::settings::item::builder("Music Directory:")
                            .description("Choose directory to scan for music.")
                            .control(
                                match self.rescan_available {
                                    true => {
                                        cosmic::widget::button::suggested("Select Folder").on_press(Message::ChooseFolder)
                                    }
                                    false => {
                                        cosmic::widget::button::suggested("Select Folder")
                                    }
                                }

                            ),
                    )
                    .add(
                        widget::settings::item::builder("Full Directory Rescan:").control(
                            match self.rescan_available && !self.config.scan_dir.is_empty() {
                                true => {

                                    widget::button::text("Rescan")
                                        .class(widget::button::ButtonClass::Destructive)
                                        .on_press(app::Message::RescanDir)
                                }
                                false => {
                                    widget::button::text("Rescan")
                                        .class(widget::button::ButtonClass::Destructive)
                                }
                            }

                        ),
                    )
                    .add(
                        widget::column::Column::with_children([
                            widget::column::Column::with_children([
                                widget::text::heading("Scan Progress: ").into(),

                                widget::text::caption(format!("{}%", (self.config.files_scanned as f32 /self.config.num_files_found as f32 * 100.0).round()))
                                    .align_x(Horizontal::Right)
                                    .into(),
                            ])
                            .into(),
                            widget::progress_bar(0.0..=self.config.num_files_found as f32, self.config.files_scanned as f32)
                                .height(space_s)
                                .into(),
                        ])
                        .spacing(space_xxs),
                    )
                    .into(),
                grid_settings
                    .title("User Interface")
                    .add(
                        widget::settings::item::builder("Grid Item Size: ")
                            .control(cosmic::widget::slider(1..=6, self.config.grid_item_size, |a| Message::GridSliderChange(a))
                                
                            )
                    )
                    .into(),

                player_settings
                    .title("Music Player")
                    .add(
                            widget::settings::item::builder(format!("App Volume: {}", self.config.volume.trunc()))
                            .control(
                                cosmic::widget::slider(0.0..=100.0, self.config.volume, |a| Message::VolumeSliderChange(a))

                            )
                    )
                    .into(),
            ])
            .spacing(space_s),
        );
        contain.into()
    }
}
