// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::CoverArt::SomeLoaded;
use crate::app::page::{list_sort_header, BodyStyle, CoverArt, Page, PageBuilder};
use crate::app::{connect_to_db, AppModel, AppTrack, Message};
use crate::config::SortOrder;
use crate::fl;
use colored::Colorize;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{widget, ContentFit, Length, Point};
use cosmic::iced_core::{Alignment, Size};
use cosmic::iced_widget::scrollable::AbsoluteOffset;
use cosmic::widget::JustifyContent;
use cosmic::{iced_core, Element, Task};
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
    pub scrollbar_id: cosmic::iced_core::widget::Id,

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

        let visible_rect = iced_core::Rectangle::new(
            iced_core::Point::new(
                f32::from(cosmic::theme::spacing().space_s),
                match self.viewport {
                    None => 1.0,
                    Some(val) => val.absolute_offset().y,
                },
            ),
            iced_core::Size::new(3.0, visible_height),
        );

        let mut tracks: Vec<Element<Message>> = vec![];

        let mut tracks_rect = iced_core::Rectangle::new(
            iced_core::Point::new(f32::from(cosmic::theme::spacing().space_s), 1.0),
            iced_core::Size::new(3.0, 64.0),
        );

        if cosmic::iced_core::mouse::Cursor::is_over(
            cosmic::iced_core::mouse::Cursor::default(),
            tracks_rect,
        ) {
            log::info!("{}", "this is true somehow".to_string().on_bright_yellow())
        }

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
                            .class(cosmic::theme::Container::Primary)
                            .into(),
                    )
                } else {
                    tracks.push(
                        cosmic::widget::container::Container::new(display_element)
                            .align_y(Vertical::Center)
                            .class(cosmic::theme::Container::List)
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
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
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
    pub fn display(self) -> Element<'static, Message> {
        cosmic::widget::column::with_children(vec![
            cosmic::widget::divider::horizontal::default().into(),
            cosmic::widget::row::with_children(vec![
                widget::column![
                    cosmic::widget::text::heading(self.title),
                    cosmic::widget::text::text(self.artist),
                    cosmic::widget::text::text(self.album_title),
                ]
                .into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::button::text("x ^v >").into(),
            ])
            .height(Length::Fixed(64.0))
            .align_y(Vertical::Center)
            .into(),
            cosmic::widget::divider::horizontal::default().into(),
        ])
        .into()
    }
}
