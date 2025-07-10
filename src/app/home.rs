use crate::app;
use crate::app::albums::Album;
use crate::app::home::HomePageState::Empty;
use crate::app::Message;
use colored::Colorize;
use cosmic::cosmic_theme::palette::chromatic_adaptation::AdaptInto;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::Alignment::Start;
use cosmic::iced::{Center, ContentFit, Length, Pixels};
use cosmic::widget::{container, list_column, JustifyContent, ListColumn};
use cosmic::{iced, iced_core, Element};
use std::fmt::{format, Alignment};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct HomePage {
    pub state: HomePageState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum HomePageState {
    #[default]
    Empty,
    Queued(Vec<HomeTrack>),
}

impl HomePage {
    pub fn new() -> Self {
        HomePage { state: Empty }
    }

    pub fn load(&self) -> Element<'static, app::Message> {
        match &self.state {
            Empty => {
                cosmic::widget::scrollable(
                    cosmic::widget::container(
                        cosmic::widget::column::with_children(vec![
                            cosmic::widget::container(cosmic::widget::column::with_children(vec![
                                cosmic::widget::flex_row(vec![
                                    cosmic::widget::container(
                                        cosmic::widget::Column::with_children(vec![
                                            cosmic::widget::text::title1("Queue Empty").into(),
                                        ]),
                                    )
                                    .into(),
                                    cosmic::widget::container(
                                        cosmic::widget::Column::with_children(vec![
                                            cosmic::widget::icon::from_name(
                                                "applications-audio-symbolic",
                                            )
                                            .size(192)
                                            .into(),
                                            cosmic::widget::icon::from_name(
                                                "default-media-symbolic",
                                            )
                                            .into(),
                                        ]),
                                    )
                                    .into(),
                                ])
                                .spacing(cosmic::theme::spacing().space_s)
                                .justify_content(JustifyContent::SpaceAround)
                                .into(),
                                cosmic::widget::container(
                                    cosmic::widget::flex_row(vec![
                                        // Media Progress
                                        cosmic::widget::row::with_children(vec![
                                            cosmic::widget::text::heading("00:00").into(),
                                            cosmic::widget::slider(0..=100, 0, |a| {
                                                Message::VolumeSliderAdjusted(a)
                                            })
                                            .width(Length::Fill)
                                            .height(31.0)
                                            .into(),
                                        ])
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
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-playback-start-symbolic",
                                                ),
                                            )
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-skip-forward-symbolic",
                                                ),
                                            )
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-playlist-no-repeat-symbolic",
                                                ),
                                            )
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                        ])
                                        .align_y(Vertical::Center)
                                        .spacing(cosmic::theme::spacing().space_xxs)
                                        .into(),
                                    ])
                                    .align_items(iced_core::Alignment::Center)
                                    .spacing(cosmic::theme::spacing().space_xs)
                                    .justify_content(JustifyContent::SpaceAround),
                                )
                                .padding(cosmic::theme::spacing().space_xxs)
                                .class(cosmic::style::Container::Secondary)
                                .into(),
                            ]))
                            .width(Length::Fill)
                            .padding(cosmic::theme::spacing().space_m)
                            .class(cosmic::theme::Container::Primary)
                            .into(),
                            cosmic::widget::container(
                                cosmic::widget::column::with_children(vec![
                                    cosmic::widget::text::heading("Up Next: ").into(),
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
            HomePageState::Queued(val) => {
                cosmic::widget::scrollable(
                    cosmic::widget::container(
                        cosmic::widget::column::with_children(vec![
                            cosmic::widget::container(cosmic::widget::column::with_children(vec![
                                cosmic::widget::flex_row(vec![
                                    cosmic::widget::row::with_children(vec![
                                        // todo: Wanted to add a nifty divider here but its not working out right now
                                        cosmic::widget::Column::with_children(vec![
                                            cosmic::widget::text::title1(format!(
                                                "Now Playing: {}",
                                                match val.get(0) {
                                                    None => {
                                                        "Nothing"
                                                    }
                                                    Some(val) => {
                                                        val.title.as_str()
                                                    }
                                                }
                                            ))
                                            .into(),
                                            cosmic::widget::text::title2(format!(
                                                "By: {}",
                                                match val.get(0) {
                                                    None => {
                                                        "Nothing"
                                                    }
                                                    Some(val) => {
                                                        val.artist.as_str()
                                                    }
                                                }
                                            ))
                                            .into(),
                                        ]).into(),
                                    ])
                                        .spacing(cosmic::theme::spacing().space_s)
                                    .into(),
                                    cosmic::widget::container(
                                        cosmic::widget::Column::with_children(vec![
                                            match val.get(0) {
                                                None => cosmic::widget::icon::from_name(
                                                    "applications-audio-symbolic",
                                                )
                                                .size(192)
                                                .into(),
                                                Some(track) => cosmic::widget::image(
                                                    track.clone().cover_art.unwrap(),
                                                )
                                                .height(192.0)
                                                .width(192.0)
                                                .content_fit(ContentFit::Contain)
                                                .border_radius([3.0, 3.0, 3.0, 3.0])
                                                .into(),
                                            },
                                        ]),
                                    )
                                    .padding(cosmic::theme::spacing().space_m)
                                    .into(),
                                ])
                                .spacing(cosmic::theme::spacing().space_s)
                                .justify_content(JustifyContent::SpaceAround)
                                .into(),
                                cosmic::widget::container(
                                    cosmic::widget::flex_row(vec![
                                        // Media Progress
                                        cosmic::widget::row::with_children(vec![
                                            cosmic::widget::text::heading("0:26").into(),
                                            cosmic::widget::slider(0..=100, 50, |a| {
                                                Message::VolumeSliderAdjusted(a)
                                            })
                                            .width(Length::Fill)
                                            .height(31.0)
                                            .into(),
                                        ])
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
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-playback-start-symbolic",
                                                ),
                                            )
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-skip-forward-symbolic",
                                                ),
                                            )
                                            .on_press(Message::SkipTrack)
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                            cosmic::widget::button::icon(
                                                cosmic::widget::icon::from_name(
                                                    "media-playlist-no-repeat-symbolic",
                                                ),
                                            )
                                            .class(cosmic::widget::button::ButtonClass::Standard)
                                            .into(),
                                        ])
                                        .align_y(Vertical::Center)
                                        .spacing(cosmic::theme::spacing().space_xxs)
                                        .into(),
                                    ])
                                    .align_items(iced_core::Alignment::Center)
                                    .spacing(cosmic::theme::spacing().space_xs)
                                    .justify_content(JustifyContent::SpaceAround),
                                )
                                .padding(cosmic::theme::spacing().space_xxs)
                                .class(cosmic::style::Container::Secondary)
                                .into(),
                            ]))
                            .width(Length::Fill)
                            .padding(cosmic::theme::spacing().space_m)
                            .class(cosmic::theme::Container::Primary)
                            .into(),
                            cosmic::widget::container(
                                cosmic::widget::column::with_children(vec![
                                    cosmic::widget::text::heading("Up Next: ").into(),
                                    listify_queue(&self.state),
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
    }
}

fn listify_queue(state: &HomePageState) -> Element<'static, Message> {
    let mut list = Some(list_column());

    if let HomePageState::Queued(val) = state {
        for (index, val) in val.iter().enumerate() {
            let name = format!("{}. {}", index + 1, val.title);

            match list.take() {
                None => {}
                Some(old_list) => {
                    list = Some(old_list.add(cosmic::widget::text(name)));
                }
            }
        }
    }

    list.unwrap().into_element()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeTrack {
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) path_buf: PathBuf,
    pub(crate) cover_art: Option<cosmic::widget::image::Handle>,
}
