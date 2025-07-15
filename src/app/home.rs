use crate::app;
use crate::app::albums::Album;
use crate::app::{AppModel, AppTrack, Message};
use colored::Colorize;
use cosmic::cosmic_theme::palette::chromatic_adaptation::AdaptInto;
use cosmic::cosmic_theme::palette::{Alpha, IntoColor, Srgba};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::Alignment::Start;
use cosmic::iced::{Center, ContentFit, Length, Pixels};
use cosmic::widget::{container, image, list_column, JustifyContent, ListColumn};
use cosmic::{iced, iced_core, Element};
use rodio::queue::queue;
use std::fmt::{format, Alignment};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cosmic::iced_core::text::Wrapping;
use humantime::format_duration;

#[derive(Debug)]
pub(crate) struct HomePage;

impl HomePage {
    pub fn load(&self, model: &AppModel) -> Element<'static, app::Message> {
                // Time ELapsed
                let mut time_elapsed = format_time(model.song_progress);

                let mut total_duration = "**:**".to_string();
                match model.song_duration {
                    None => {
                        
                    }
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
                        cover = format_cover_page(&model.queue.get(0).unwrap().title, &model.queue.get(0).unwrap().artist, Some(&model.queue.get(0).unwrap().album_title), &model.queue.get(0).unwrap().cover_art);
                    }
                }
                
                
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
                                                    |a| Message::VolumeSliderAdjusted(a),
                                                )
                                                
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
                                                .class(
                                                    cosmic::widget::button::ButtonClass::Standard,
                                                )
                                                .into(),
                                                cosmic::widget::button::icon(
                                                    cosmic::widget::icon::from_name(
                                                        "media-playback-start-symbolic",
                                                    ),
                                                )
                                                .on_press(Message::PlayPause)
                                                .class(
                                                    cosmic::widget::button::ButtonClass::Standard,
                                                )
                                                .into(),
                                                cosmic::widget::button::icon(
                                                    cosmic::widget::icon::from_name(
                                                        "media-skip-forward-symbolic",
                                                    ),
                                                )
                                                .on_press(Message::SkipTrack)
                                                .class(
                                                    cosmic::widget::button::ButtonClass::Standard,
                                                )
                                                .into(),
                                                cosmic::widget::button::icon(
                                                    cosmic::widget::icon::from_name(
                                                        "media-playlist-no-repeat-symbolic",
                                                    ),
                                                )
                                                .class(
                                                    cosmic::widget::button::ButtonClass::Standard,
                                                )
                                                .into(),
                                            ])

                                                .width(Length::Shrink)
                                            .align_y(Vertical::Center)
                                            .spacing(cosmic::theme::spacing().space_xxxs)
                                            .into(),
                                        ])
                                        .spacing(cosmic::theme::spacing().space_xs)
                                    )
                                    .padding(cosmic::theme::spacing().space_xxs)
                                    .class(cosmic::style::Container::Secondary)
                                    .into(),
                                ])
                                .spacing(cosmic::theme::spacing().space_xs),
                            )
                            .width(Length::Fill)
                            .padding(cosmic::theme::spacing().space_xs)
                            .class(cosmic::theme::Container::Primary)
                            .into(),
                            cosmic::widget::container(
                                cosmic::widget::column::with_children(vec![
                                    cosmic::widget::text::heading("Queue: ").into(),
                                    listify_queue(&model.queue),
                                ])
                                .spacing(cosmic::theme::spacing().space_xxs),
                            )
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

fn listify_queue(queue: &Vec<AppTrack>) -> Element<'static, Message> {
    let mut list = Some(list_column());

    
        let length = queue.len();

        let mut queue_num = 0;
        for item in 1..length {
            let name = format!("{}. {}", queue_num + 1, queue.get(item).unwrap().title);

            match list.take() {
                None => {}
                Some(old_list) => {
                    list = Some(old_list.add(cosmic::widget::text(name)));
                }
            }

            queue_num += 1;
    }

    list.unwrap().into_element()
}

fn format_cover_page(title: &String, artist: &String, album: Option<&String>, handle: &Option<image::Handle>) -> Element<'static, Message> {
    const COVER_ART_SIZE: u32 = 192;
    
    let size = COVER_ART_SIZE + cosmic::theme::spacing().space_l as u32;
    
    
    cosmic::widget::row::with_children(vec![
            cosmic::widget::container(
                cosmic::widget::Column::with_children(vec![match handle
                {
                    None => cosmic::widget::icon::from_name(
                        "applications-audio-symbolic",
                    )
                        .size(192)
                        .into(),
                    Some(track) => cosmic::widget::image(
                        track,
                    )
                        //todo make this customizable
                        .content_fit(ContentFit::ScaleDown)
                        .border_radius([12.0, 12.0, 12.0, 12.0])
                        .into(),
                }]).max_width(Pixels(COVER_ART_SIZE as f32)),
            )
                .align_x(Horizontal::Right)
                .width(Length::FillPortion(2))
                .into(),
            
            cosmic::widget::Column::with_children(vec![
                cosmic::widget::text::title3(format!(
                    "{}",
                    title
                ))
                    .wrapping(Wrapping::WordOrGlyph)
                    .into(),
                cosmic::widget::text::title4(format!(
                    "{}",
                    artist
                ))
                    .into(),

                cosmic::widget::text::title4(format!(
                    "{}",
                    album.unwrap_or(&String::new())
                )).into()

 
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


