// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::CoverArt::SomeLoaded;
use crate::app::page::{list_sort_header, BodyStyle, CoverArt, Page, PageBuilder};
use crate::app::{connect_to_db, AppModel, AppTrack, Message, TrackType};
use crate::config::SortOrder;
use crate::fl;
use colored::Colorize;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit};
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{widget, ContentFit, Length, Point};
use cosmic::widget::JustifyContent;
use cosmic::{Element, Task};
use rayon::iter::IntoParallelIterator;
use rusqlite::fallible_iterator::FallibleIterator;
use std::cell::Cell;
use std::ops::Div;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use symphonia::core::conv::IntoSample;

#[derive(Debug, Clone)]
pub struct TrackPage {
    pub tracks: Arc<RwLock<Vec<AppTrack>>>,
    pub search: Vec<SearchResult>,
    pub SearchTerm: String,
    pub page_state: TrackPageState,
    pub viewport: Option<Viewport>,
    pub load_depth: u32,
    pub scrollbar_id: cosmic::iced::widget::Id,

    pub search_by_artist: bool,
    pub search_by_album: bool,
    pub search_by_title: bool,
}

#[derive(Debug, Clone)]
pub enum TrackPageState {
    Waiting,
    Loading,
    Loaded,
    Search,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub tracks_index: usize,
    pub score: u32,
}

// Page Definition
impl Page for TrackPage {
    fn title(&self) -> String {
        String::from(fl!("TrackLibrary"))
    }
    fn body(&self, model: &AppModel) -> Element<Message> {
        if let TrackPageState::Waiting = self.page_state {
            return cosmic::widget::text::heading("Loading...").into();
        }

        let visible_height = match self.viewport {
            None => 0.0,
            Some(val) => val.bounds().height,
        };

        let visible_rect = cosmic::iced::Rectangle::new(
            cosmic::iced::Point::new(
                f32::from(cosmic::theme::spacing().space_s),
                match self.viewport {
                    None => 1.0,
                    Some(val) => val.absolute_offset().y,
                },
            ),
            cosmic::iced::Size::new(3.0, visible_height),
        );

        let mut tracks: Vec<Element<Message>> = vec![];

        let mut tracks_rect = cosmic::iced::Rectangle::new(
            cosmic::iced::Point::new(f32::from(cosmic::theme::spacing().space_s), 1.0),
            cosmic::iced::Size::new(3.0, 64.0),
        );

        let mut loaded = 0;
        for (index, track) in self.tracks.clone().read().unwrap().iter().enumerate() {
            loaded += 1;
            tracks_rect.y += 64.0;

            if tracks_rect.intersects(&visible_rect) {
                let owned_track = track.clone();
                let display_element: Element<Message> = owned_track.display().into();

                if index % 2 == 0 {
                    tracks.push(
                        cosmic::widget::container::Container::new(display_element)
                            .align_y(Vertical::Center)
                            .into(),
                    )
                } else {
                    tracks.push(
                        cosmic::widget::container::Container::new(display_element)
                            .align_y(Vertical::Center)
                            .into(),
                    )
                }
            } else {
                tracks.push(
                    cosmic::widget::column::with_children(vec![])
                        .height(Length::Fixed(64.0))
                        .into(),
                );
            }
        }

        cosmic::widget::column::with_children(vec![
            cosmic::widget::column::with_children(tracks).into()
        ])
        .into()
    }

    fn body_style(&self) -> BodyStyle {
        BodyStyle::List
    }
}

impl TrackPage {
    pub fn new() -> TrackPage {
        TrackPage {
            tracks: Arc::new(RwLock::from(vec![])),
            search: vec![],
            SearchTerm: String::from(""),
            page_state: TrackPageState::Loading,
            viewport: None,
            load_depth: 0,
            scrollbar_id: cosmic::iced::widget::Id::unique(),
            search_by_artist: false,
            search_by_album: false,
            search_by_title: false,
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.page(model)
    }
    pub fn load_page_data(&self) -> Task<cosmic::Action<Message>> {
        return cosmic::Task::future( async move {
            let conn = connect_to_db();

            let mut stmt = conn.prepare(
                "
                                select track.id as id, track.name as title, art.name as artist, track.path, a.name as album_title
                                from track
                                    left join main.album_tracks at on track.id = at.track_id
                                    left join main.artists art on track.artist_id = art.id
                                    left join main.album a on at.album_id = a.id;
                            ").unwrap();

            let tracks = stmt.query_map([], |row| {
                Ok(
                    AppTrack {
                        id: row.get("id").unwrap_or(0),
                        title: row
                            .get("title")
                            .unwrap_or("N/A".to_string()),
                        artist: row
                            .get("artist")
                            .unwrap_or("N/A".to_string()),
                        album_title: row
                            .get("album_title")
                            .unwrap_or("N/A".to_string()),
                        path_buf: PathBuf::from(
                            row.get::<&str, String>("path")
                                .expect("This should never happen"),
                        ),
                        cover_art: CoverArt::None,
                    }
                )
            }).expect("Should never break");


            let tracks = tracks
                .filter_map(|a| a.ok())
                .collect::<Vec<AppTrack>>();

            log::info!("Loading track data from the database done ");
            // log::info!("| time since entering the page {}ms", timer.elapsed().as_millis());
            Message::TrackDataReceived(tracks)
        }).map(cosmic::Action::App);
    }
}

impl AppTrack {
    pub fn display<'a>(self) -> Element<'a, Message> {
        let space_main = 300.0;
        let space_mod = 150.0;
        cosmic::widget::column::with_children(vec![cosmic::iced::widget::hover(
            // Normal Display
            cosmic::widget::container(cosmic::widget::column![
                cosmic::widget::divider::horizontal::default(),
                cosmic::widget::row::with_children(vec![
                    widget::column![
                        cosmic::widget::text::heading(self.title.to_string())
                            .width(Length::Fixed(space_main))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                        cosmic::widget::text::text(self.artist.to_string())
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                            .width(Length::Fixed(space_main)),
                        cosmic::widget::text::text(self.album_title.to_string())
                            .width(Length::Fixed(space_main))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                    ]
                    .into(),
                    // todo: custom extra info sections, with sorting capabilities
                    // cosmic::widget::button::text("Mod Entry 1")
                    //     .width(Length::Fixed(space_mod))
                    //     .into(),
                    // cosmic::widget::button::text("Mod Entry 2")
                    //     .width(Length::Fixed(space_mod))
                    //     .into(),
                    cosmic::widget::space().width(Length::Fill).into(),
                ])
                .padding(cosmic::iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_xxs,
                ]))
                .height(Length::Fixed(64.0))
                .align_y(Vertical::Center),
                cosmic::widget::divider::horizontal::default(),
            ]),
            // Display on hover, where the controls should be
            // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
            cosmic::widget::container(cosmic::widget::column![
                cosmic::widget::divider::horizontal::light(),
                cosmic::widget::row::with_children(vec![
                    widget::column![
                        cosmic::widget::text::heading(self.title.to_string())
                            .width(Length::FillPortion(1))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                        cosmic::widget::text::text(self.artist.to_string())
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                            .width(Length::FillPortion(1)),
                        cosmic::widget::text::text(self.album_title.to_string())
                            .width(Length::FillPortion(1))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                    ]
                    .into(),
                    cosmic::widget::row::with_children(vec![
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("playlist-symbolic"),
                            ))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Add to playlist")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("list-add-symbolic"),
                            ))
                            .on_press(Message::AddTrackById(self.id))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Add to queue")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("media-playback-start-symbolic"),
                            ))
                            .on_press(Message::PlayTrackById(self.id))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Play now")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_xxxs)
                    .into(), //right
                ])
                .spacing(cosmic::theme::spacing().space_s)
                .padding(cosmic::iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_xxs,
                ]))
                .height(Length::Fixed(64.0))
                .align_y(Vertical::Center),
                cosmic::widget::divider::horizontal::light(),
            ])
            .class(cosmic::theme::Container::Card),
        )])
        .into()
    }
}
