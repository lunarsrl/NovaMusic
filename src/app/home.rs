// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app;
use crate::app::playback::PlaybackManager;
use crate::app::{AppModel, LoopState, Message};
use crate::fl;

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::Alignment::Start;
use cosmic::iced::{ContentFit, Length, Pixels};
use cosmic::{iced, Element};

use cosmic::iced_core::text::Wrapping;
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::widget::{image, list_column};

#[derive(Debug)]
pub(crate) struct HomePage {
    pub viewport: Option<Viewport>,
}

impl HomePage {
    pub fn load_page<'a>(&self, model: &'a AppModel) -> Element<'a, Message> {
        let time_elapsed = format_time(model.song_progress);
        let total_duration = model
            .song_duration
            .map(format_time)
            .unwrap_or_else(|| "**:**".to_string());

        let cover = match model.playback_manager.current_track() {
            None => format_cover_page(&"".to_string(), &"".to_string(), None, &None),
            Some(track) => format_cover_page(
                &track.title,
                &track.artist,
                Some(&track.album_title),
                &track.cover_art,
            ),
        };

        let play_pause_button: Element<Message> = if model.playback_manager.is_empty() {
            model.sink.clear();
            cosmic::widget::button::icon(if model.sink.is_paused() {
                cosmic::widget::icon::from_name("media-playback-start-symbolic")
            } else {
                cosmic::widget::icon::from_name("media-playback-pause-symbolic")
            })
            .into()
        } else {
            cosmic::widget::button::icon(if model.sink.is_paused() {
                cosmic::widget::icon::from_name("media-playback-start-symbolic")
            } else {
                cosmic::widget::icon::from_name("media-playback-pause-symbolic")
            })
            .on_press(Message::PlayPause)
            .into()
        };

        let history_label = if model.show_history {
            "Hide History"
        } else {
            "Show History"
        };

        cosmic::widget::container(
            cosmic::widget::scrollable(
                cosmic::widget::container(
                    cosmic::widget::column::with_children(vec![
                        cosmic::widget::container(
                            cosmic::widget::column::with_children(vec![
                                cover,
                                cosmic::widget::container(
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::row::with_children(vec![
                                            cosmic::widget::text::heading(time_elapsed).into(),
                                            cosmic::widget::slider(
                                                0.0..=model.song_duration.unwrap_or(1.0),
                                                model.song_progress,
                                                |a| Message::SeekTrack(a),
                                            )
                                            .on_release(Message::SeekFinished)
                                            .height(31.0)
                                            .into(),
                                            cosmic::widget::text::heading(total_duration).into(),
                                        ])
                                        .width(Length::Fill)
                                        .align_y(Vertical::Center)
                                        .spacing(cosmic::theme::spacing().space_xxs)
                                        .into(),
                                        cosmic::widget::row::with_children(vec![
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-skip-backward-symbolic",
                                                ),
                                            )
                                            .on_press(Message::PreviousTrack)
                                            .into(),
                                            play_pause_button,
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-skip-forward-symbolic",
                                                ),
                                            )
                                            .on_press(Message::SkipTrack)
                                            .into(),
                                            {
                                                let loop_state =
                                                    model.playback_manager.loop_state();
                                                cosmic::widget::button::icon(match loop_state {
                                                    LoopState::LoopingTrack => {
                                                        cosmic::widget::icon::from_name(
                                                            "media-playlist-repeat-song-symbolic",
                                                        )
                                                    }
                                                    LoopState::LoopingQueue => {
                                                        cosmic::widget::icon::from_name(
                                                            "media-playlist-repeat-symbolic",
                                                        )
                                                    }
                                                    LoopState::NotLooping
                                                    | LoopState::Unavailable => {
                                                        cosmic::widget::icon::from_name(
                                                            "media-playlist-no-repeat-symbolic",
                                                        )
                                                    }
                                                })
                                                .class(match loop_state {
                                                    LoopState::LoopingTrack
                                                    | LoopState::LoopingQueue => {
                                                        cosmic::theme::Button::Suggested
                                                    }
                                                    _ => cosmic::theme::Button::Icon,
                                                })
                                                .on_press_maybe(match loop_state {
                                                    LoopState::Unavailable => None,
                                                    _ => Some(Message::ChangeLoopState),
                                                })
                                                .into()
                                            },
                                        ])
                                        .width(Length::Shrink)
                                        .align_y(Vertical::Center)
                                        .spacing(cosmic::theme::spacing().space_xxxs)
                                        .into(),
                                    ])
                                    .spacing(cosmic::theme::spacing().space_xs),
                                )
                                .padding(cosmic::theme::spacing().space_xxs)
                                .class(cosmic::style::Container::Card)
                                .into(),
                            ])
                            .spacing(cosmic::theme::spacing().space_xs),
                        )
                        .width(Length::Fill)
                        .padding(cosmic::theme::spacing().space_xxs)
                        .class(cosmic::theme::Container::Primary)
                        .into(),
                        cosmic::widget::container(
                            cosmic::widget::column::with_children(vec![
                                cosmic::widget::row::with_children(vec![
                                    cosmic::widget::text::heading(fl!("Queue")).center().into(),
                                    cosmic::widget::horizontal_space().into(),
                                    cosmic::widget::button::text(history_label)
                                        .class(cosmic::widget::button::ButtonClass::Standard)
                                        .on_press(Message::ToggleHistory)
                                        .into(),
                                    cosmic::widget::button::text(fl!("CreatePlaylist"))
                                        .class(cosmic::widget::button::ButtonClass::Standard)
                                        .on_press(Message::AddToPlaylist)
                                        .into(),
                                    cosmic::widget::button::text(fl!("ClearAll"))
                                        .class(cosmic::widget::button::ButtonClass::Destructive)
                                        .on_press(Message::ClearQueue)
                                        .into(),
                                ])
                                .align_y(Vertical::Center)
                                .spacing(cosmic::theme::spacing().space_xxs)
                                .into(),
                                cosmic::widget::divider::horizontal::default().into(),
                                {
                                    let mut sections = vec![
                                        cosmic::widget::text::heading("Up Next").into(),
                                        listify_up_next_section(&model.playback_manager),
                                        cosmic::widget::divider::horizontal::default().into(),
                                        cosmic::widget::text::heading("Context").into(),
                                        listify_context_section(&model.playback_manager),
                                    ];

                                    if model.show_history {
                                        sections.push(
                                            cosmic::widget::divider::horizontal::default().into(),
                                        );
                                        sections
                                            .push(cosmic::widget::text::heading("History").into());
                                        sections
                                            .push(listify_history_section(&model.playback_manager));
                                    }

                                    cosmic::widget::column::with_children(sections)
                                        .spacing(cosmic::theme::spacing().space_xxs)
                                        .into()
                                },
                            ])
                            .spacing(cosmic::theme::spacing().space_xxs),
                        )
                        .class(cosmic::theme::Container::Primary)
                        .padding(cosmic::theme::spacing().space_xxs)
                        .into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_m),
                )
                .align_y(Start)
                .width(Length::Fill)
                .padding(iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_m,
                ])),
            )
            .height(Length::Fill)
            .width(Length::Fill),
        )
        .padding(iced::core::padding::Padding::from([
            0,
            0,
            cosmic::theme::spacing().space_xxs,
            0,
        ]))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}

pub fn listify_up_next_section(playback_manager: &PlaybackManager) -> Element<'static, Message> {
    let mut list = list_column();
    let indices = playback_manager.upcoming_up_next_global_indices();

    if indices.is_empty() {
        list = list.add(cosmic::widget::text("No upcoming tracks"));
        return list.into_element();
    }

    for (i, global_idx) in indices.iter().enumerate() {
        if let Some(track) = playback_manager.track_by_global_index(*global_idx) {
            let name = format!("{}. {}", i + 1, track.title);
            list = list.add(
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text(name).into(),
                    cosmic::widget::horizontal_space().into(),
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "window-close-symbolic",
                    ))
                    .on_press(Message::RemoveTrack(*global_idx))
                    .into(),
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "media-playback-start-symbolic",
                    ))
                    .on_press(Message::ChangeActiveTrack(*global_idx))
                    .into(),
                ])
                .align_y(Vertical::Center)
                .spacing(cosmic::theme::spacing().space_xxxs),
            );
        }
    }

    list.into_element()
}

pub fn listify_context_section(playback_manager: &PlaybackManager) -> Element<'static, Message> {
    let mut list = list_column();
    let indices = playback_manager.upcoming_context_global_indices();

    if indices.is_empty() {
        list = list.add(cosmic::widget::text("No upcoming tracks"));
        return list.into_element();
    }

    for (i, global_idx) in indices.iter().enumerate() {
        if let Some(track) = playback_manager.track_by_global_index(*global_idx) {
            let name = format!("{}. {}", i + 1, track.title);
            list = list.add(
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text(name).into(),
                    cosmic::widget::horizontal_space().into(),
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "window-close-symbolic",
                    ))
                    .on_press(Message::RemoveTrack(*global_idx))
                    .into(),
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "media-playback-start-symbolic",
                    ))
                    .on_press(Message::ChangeActiveTrack(*global_idx))
                    .into(),
                ])
                .align_y(Vertical::Center)
                .spacing(cosmic::theme::spacing().space_xxxs),
            );
        }
    }

    list.into_element()
}

pub fn listify_history_section(playback_manager: &PlaybackManager) -> Element<'static, Message> {
    let mut list = list_column();
    let indices = playback_manager.played_global_indices();

    if indices.is_empty() {
        list = list.add(cosmic::widget::text("No history yet"));
        return list.into_element();
    }

    for (i, global_idx) in indices.iter().enumerate() {
        if let Some(track) = playback_manager.track_by_global_index(*global_idx) {
            let name = format!("{}. {}", i + 1, track.title);
            list = list.add(
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text(name).into(),
                    cosmic::widget::horizontal_space().into(),
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "media-playback-start-symbolic",
                    ))
                    .on_press(Message::ChangeActiveTrack(*global_idx))
                    .into(),
                ])
                .align_y(Vertical::Center)
                .spacing(cosmic::theme::spacing().space_xxxs),
            );
        }
    }

    list.into_element()
}

pub(crate) fn format_cover_page(
    title: &String,
    artist: &String,
    album: Option<&String>,
    handle: &Option<image::Handle>,
) -> Element<'static, Message> {
    const COVER_ART_SIZE: u32 = 192;

    cosmic::widget::row::with_children(vec![
        cosmic::widget::container(
            cosmic::widget::Column::with_children(vec![match handle {
                None => cosmic::widget::icon::from_name("applications-audio-symbolic")
                    .size(192)
                    .into(),
                Some(track) => cosmic::widget::image(track)
                    .content_fit(ContentFit::ScaleDown)
                    .border_radius([12.0, 12.0, 12.0, 12.0])
                    .into(),
            }])
            .max_width(Pixels(COVER_ART_SIZE as f32)),
        )
        .align_x(Horizontal::Right)
        .width(Length::FillPortion(2))
        .into(),
        cosmic::widget::Column::with_children(vec![
            cosmic::widget::text::title3(format!("{}", title))
                .wrapping(Wrapping::WordOrGlyph)
                .into(),
            cosmic::widget::text::title4(format!("{}", artist)).into(),
            cosmic::widget::text::title4(format!("{}", album.unwrap_or(&String::new()))).into(),
        ])
        .spacing(cosmic::theme::spacing().space_s)
        .width(Length::FillPortion(2))
        .into(),
    ])
    .spacing(cosmic::theme::spacing().space_l)
    .into()
}

pub fn format_time(mut seconds: f64) -> String {
    let mut minutes = 0;
    let seconds_final = (seconds % 60.0) as u64;

    loop {
        seconds -= 60.0;
        if seconds < 0.0 {
            break;
        }
        minutes += 1;
    }

    let seconds_format = if seconds_final < 10 {
        format!("0{}", seconds_final)
    } else {
        seconds_final.to_string()
    };

    let minute_format = if minutes < 10 {
        format!("0{}", minutes)
    } else {
        minutes.to_string()
    };

    format!("{}:{}", minute_format, seconds_format)
}
