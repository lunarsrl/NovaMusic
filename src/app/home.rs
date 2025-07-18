use crate::app;
use crate::app::albums::Album;
use crate::app::{AppModel, AppTrack, LoopState, Message};
use colored::Colorize;
use cosmic::cosmic_theme::palette::chromatic_adaptation::AdaptInto;
use cosmic::cosmic_theme::palette::{Alpha, IntoColor, Srgba};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::Alignment::Start;
use cosmic::iced::{color, Center, ContentFit, Length, Pixels};
use cosmic::widget::{container, image, list_column, JustifyContent, ListColumn};
use cosmic::{iced, iced_core, Element};
use rodio::queue::queue;
use std::fmt::{format, Alignment};
use std::future::IntoFuture;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cosmic::iced_core::text::Wrapping;
use cosmic::style::Text::Color;

use humantime::format_duration;

#[derive(Debug)]
pub(crate) struct HomePage;

impl HomePage {
    pub fn load(&self, model: &AppModel) -> Element<'static, app::Message> {
        // Time ELapsed
        let mut time_elapsed = format_time(model.song_progress);

        let mut total_duration = "**:**".to_string();
        match model.song_duration {
            None => {}
            Some(val) => {
                total_duration = format_time(val);
            }
        };

        let mut cover;
        match model.queue.is_empty() {
            true => {
                cover = format_cover_page(&"None".to_string(), &"None".to_string(), None, &None);
            }
            false => {
                cover = format_cover_page(
                    &model.queue.get(model.queue_pos as usize).unwrap().title,
                    &model.queue.get(model.queue_pos as usize).unwrap().artist,
                    Some(
                        &model
                            .queue
                            .get(model.queue_pos as usize)
                            .unwrap()
                            .album_title,
                    ),
                    &model.queue.get(model.queue_pos as usize).unwrap().cover_art,
                );
            }
        }

        let play_pause_button: cosmic::Element<Message> = match model.queue.is_empty() {
            true => {
                model.sink.clear();
                cosmic::widget::button::icon(
                    match model.sink.is_paused() {
                        true => cosmic::widget::icon::from_name(
                            "media-playback-start-symbolic",
                        ),
                        false => cosmic::widget::icon::from_name(
                            "media-playback-pause-symbolic",
                        ),
                    },
                )
                    .into()
            }
            false => {
                cosmic::widget::button::icon(
                    match model.sink.is_paused() {
                        true => cosmic::widget::icon::from_name(
                            "media-playback-start-symbolic",
                        ),
                        false => cosmic::widget::icon::from_name(
                            "media-playback-pause-symbolic",
                        ),
                    },
                )
                    .on_press(Message::PlayPause)
                    .into()
            }
        };



        // Actual contents
        cosmic::widget::scrollable(
            cosmic::widget::container(
                cosmic::widget::column::with_children(vec![
                    cosmic::widget::container(
                        cosmic::widget::column::with_children(vec![
                            // HomePage Cover
                            cover,
                            // HomePage Cover
                            cosmic::widget::container(
                                cosmic::widget::row::with_children(vec![
                                    // Media Progress
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
                                        cosmic::widget::text::heading(format!(
                                            "{}",
                                            total_duration
                                        ))
                                        .into(),
                                    ])
                                    .width(Length::Fill)
                                    .align_y(Vertical::Center)
                                    .spacing(cosmic::theme::spacing().space_xxs)
                                    .into(),
                                    // Media Controls
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::button::icon(
                                            cosmic::widget::icon::from_name(
                                                "media-skip-backward-symbolic",
                                            ),
                                        )
                                        .on_press(Message::PreviousTrack)
                                        .into(),
                                        // PLAY OR PAUSE
                                            play_pause_button,
                                        // PLAY OR PAUSE
                                        cosmic::widget::button::icon(
                                            cosmic::widget::icon::from_name(
                                                "media-skip-forward-symbolic",
                                            ),
                                        )
                                        .on_press(Message::SkipTrack)
                                        .into(),
                                        cosmic::widget::button::icon(match model.loop_state {
                                            LoopState::LoopingTrack => {
                                                cosmic::widget::icon::from_name(
                                                    "media-playlist-repeat-song-symbolic",
                                                )
                                            }
                                            LoopState::LoopingQueue => {
                                                cosmic::widget::icon::from_name(
                                                    "media-playlist-no-repeat-symbolic",
                                                )
                                            }
                                            LoopState::NotLooping => {
                                                cosmic::widget::icon::from_name(
                                                    "media-playlist-consecutive-symbolic",
                                                )
                                            }
                                        })
                                        .on_press(Message::ChangeLoopState)
                                        .into(),
                                    ])
                                    .width(Length::Shrink)
                                    .align_y(Vertical::Center)
                                    .spacing(cosmic::theme::spacing().space_xxxs)
                                    .into(),
                                ])
                                .spacing(cosmic::theme::spacing().space_xs),
                            )
                            .padding(cosmic::theme::spacing().space_xxs)
                            .class(cosmic::style::Container::Secondary)
                            .into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_xs),
                    )
                    .width(Length::Fill)
                    .padding(cosmic::theme::spacing().space_xxs)
                    .class(cosmic::theme::Container::Primary)
                    .into(),
                    cosmic::widget::container(cosmic::widget::column::with_children(vec![
                        cosmic::widget::container(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading("Queue: ").center().into(),
                                cosmic::widget::horizontal_space().into(),
                                cosmic::widget::button::text("Clear All")
                                    .class(cosmic::widget::button::ButtonClass::Destructive)
                                    .on_press(Message::ClearQueue)
                                    .into(),
                                // todo: create playlist from queue
                                // cosmic::widget::button::text("Create Playlist")
                                //     .class(cosmic::widget::button::ButtonClass::Standard)
                                //     .on_press(Message::CreatePlaylist)
                                //     .into(),
                            ])
                            .align_y(Vertical::Center)
                            .spacing(cosmic::theme::spacing().space_xxs),
                        )
                        .class(cosmic::theme::Container::Primary)
                        .padding(cosmic::theme::spacing().space_xxs)
                        .into(),
                        listify_queue(&model.queue, model.queue_pos as usize),
                    ]))
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
        .into()
    }
}

fn listify_queue(queue: &Vec<AppTrack>, active: usize) -> Element<'static, Message> {
    let mut list = Some(list_column());

    for (index, item) in queue.iter().enumerate() {
        let name = format!("{}. {}", index + 1, item.title);

        match list.take() {
            None => {}
            Some(old_list) => {
                if index == active {
                    list = Some(
                        old_list.add(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text(name).into(),
                                cosmic::widget::horizontal_space().into(),
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "window-close-symbolic",
                                ))
                                    .on_press(Message::RemoveSongInQueue(index))
                                    .into(),
                                cosmic::widget::text("Now Playing")
                                    .class(cosmic::theme::Text::Accent)
                                    .into(),
                            ])
                            .align_y(Vertical::Center)
                            .spacing(cosmic::theme::spacing().space_xxxs),
                        ),
                    );
                } else {
                    list = Some(
                        old_list.add(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text(name).into(),
                                cosmic::widget::horizontal_space().into(),
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "window-close-symbolic",
                                ))
                                .on_press(Message::RemoveSongInQueue(index))
                                .into(),
                                cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                    cosmic::widget::icon::from_name(
                                        "media-playback-start-symbolic",
                                    ),
                                ))
                                .on_press(Message::ChangeActiveInQueue(index))
                                .into(),
                            ])
                            .align_y(Vertical::Center)
                            .spacing(cosmic::theme::spacing().space_xxxs),
                        ),
                    )
                }
            }
        }
    }

    list.unwrap().into_element()
}

fn format_cover_page(
    title: &String,
    artist: &String,
    album: Option<&String>,
    handle: &Option<image::Handle>,
) -> Element<'static, Message> {
    const COVER_ART_SIZE: u32 = 192;

    let size = COVER_ART_SIZE + cosmic::theme::spacing().space_l as u32;

    cosmic::widget::row::with_children(vec![
        cosmic::widget::container(
            cosmic::widget::Column::with_children(vec![match handle {
                None => cosmic::widget::icon::from_name("applications-audio-symbolic")
                    .size(192)
                    .into(),
                Some(track) => cosmic::widget::image(track)
                    //todo make this customizable
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
fn format_time(mut seconds: f64) -> String {
    let mut minutes = 0;

    let seconds_final = (seconds % 60.0) as u64;

    loop {
        seconds -= 60.0;
        if seconds < 0.0 {
            break;
        }
        minutes += 1;
    }

    let mut seconds_format = "".to_string();
    if seconds_final < 10 {
        seconds_format = format!("0{}", seconds_final.to_string())
    } else {
        seconds_format = seconds_final.to_string()
    }

    let mut minute_format = "".to_string();
    if minutes < 10 {
        minute_format = format!("0{}", minutes.to_string())
    } else {
        minute_format = minutes.to_string()
    }

    return format!("{}:{}", minute_format, seconds_format);
}
