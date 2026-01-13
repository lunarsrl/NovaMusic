// SPDX-License-Identifier: GPL-2.0-or-later

use std::cell::Cell;
use std::ops::Div;
use std::path::PathBuf;
use crate::app::page::CoverArt::SomeLoaded;
use crate::app::page::{CoverArt, Page, PageBuilder};
use crate::app::{connect_to_db, AppModel, AppTrack, Message};
use crate::fl;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{widget, ContentFit, Length, Point};
use cosmic::{iced_core, Element, Task};
use std::sync::{Arc, Mutex, RwLock};
use cosmic::iced_core::{Alignment, Size};
use cosmic::iced_widget::scrollable::AbsoluteOffset;
use cosmic::widget::JustifyContent;
use rayon::iter::IntoParallelIterator;
use rusqlite::fallible_iterator::FallibleIterator;

#[derive(Debug, Clone)]
pub struct TrackPage {
    pub tracks: Arc<RwLock<Vec<AppTrack>>>,
    pub search: Vec<SearchResult>,
    pub SearchTerm: String,
    pub track_page_state: TrackPageState,
    pub viewport: Option<Viewport>,
    pub load_depth: u32,
    pub scrollbar_id: cosmic::iced_core::widget::Id,

    pub search_by_artist: bool,
    pub search_by_album: bool,
    pub search_by_title: bool,
    pub size_opt: Cell<Option<Size>>,

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
    fn body(&self) -> Element<Message> {
        if let TrackPageState::Waiting = self.track_page_state {
            return cosmic::widget::text::heading("Loading...").into()
        }

        let visible_height = match self.viewport {
            None => {
                log::info!("Using None height");
                250.0
            }
            Some(val) => {
                log::info!("Using bounds height");
                val.bounds().height
            }
        };

        let visible_rect = iced_core::Rectangle::new(
            iced_core::Point::new(
                f32::from(cosmic::theme::spacing().space_s),
                match self.viewport {
                    None => {
                        1.0

                    }
                    Some(val) => {
                        log::info!("height: {} width: {} \n x: {} y: {}", self.viewport.unwrap().bounds().height, self.viewport.unwrap().bounds().width, self.viewport.unwrap().bounds().x, self.viewport.unwrap().bounds().y);
                        val.absolute_offset().y
                    }
                }
            ),
            iced_core::Size::new(3.0, visible_height)
        );


        let mut tracks : Vec<Element<Message>> = vec![];

        let mut tracks_rect = iced_core::Rectangle::new(
            iced_core::Point::new(f32::from(cosmic::theme::spacing().space_s), 5.0),
            iced_core::Size::new(3.0, 2.0)
        );

        if cosmic::iced_core::mouse::Cursor::is_over(cosmic::iced_core::mouse::Cursor::default(), tracks_rect) {
            log::info!("this is true somehow")
        }

        let mut loaded = 0;
        log::info!("pre loading -----------------");
        for (index, track) in self.tracks.clone().read().unwrap().iter().enumerate() {
            loaded += 1;
            tracks_rect.y += 64.0;

            if tracks_rect.intersects(&visible_rect) {
                let owned_track = track.clone();
                let display_element: Element<Message> = owned_track.display().into();

                if index % 2 == 0 {
                    tracks.push(
                        cosmic::widget::container::Container::new(display_element).align_y(Vertical::Center).class(cosmic::theme::Container::Primary).into()
                    )
                } else {
                    tracks.push(
                        cosmic::widget::container::Container::new(display_element).align_y(Vertical::Center).class(cosmic::theme::Container::List).into()
                    )
                }
            } else {
                log::info!("Loaded {} elements", loaded);
                return cosmic::widget::container(
                    cosmic::widget::scrollable(
                        cosmic::widget::column::with_children(vec![

                            cosmic::widget::flex_row(vec![
                                cosmic::widget::button::custom(
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::text::heading("Name").into(),
                                        cosmic::widget::icon::from_name("pan-down-symbolic").into()
                                    ]).align_y(Vertical::Center)
                                ).class(cosmic::theme::Button::Text).into(),
                                cosmic::widget::button::custom(
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::text::heading("Field 1").into(),
                                    ]).align_y(Vertical::Center)
                                ).class(cosmic::theme::Button::Text).into(),
                                cosmic::widget::button::custom(
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::text::heading("Field 2 ").into(),
                                    ]).align_y(Vertical::Center)
                                ).class(cosmic::theme::Button::Text).into(),


                            ]).justify_content(JustifyContent::SpaceBetween).align_items(Alignment::Center).into(),
                            cosmic::widget::divider::horizontal::default().into(),

                            cosmic::widget::column::with_children(
                                tracks
                            ).into()
                        ])
                    )
                ).into()

            }
        }
        log::info!("Loaded all elements");
        return cosmic::widget::container(
            cosmic::widget::scrollable(
                cosmic::widget::column::with_children(vec![
                    // sorting header thing
                    cosmic::widget::row::with_children(vec![
                        cosmic::widget::button::custom(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading("Track Info").into(),
                                cosmic::widget::icon::from_name("pan-down-symbolic").into()
                            ]).align_y(Vertical::Center)
                        ).into(),
                        cosmic::widget::button::custom(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading("editable field 1").into(),
                                cosmic::widget::icon::from_name("pan-down-symbolic").into()
                            ]).align_y(Vertical::Center)
                        ).into(),
                        cosmic::widget::button::custom(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading("editable field 2 ").into(),
                                cosmic::widget::icon::from_name("pan-down-symbolic").into()
                            ]).align_y(Vertical::Center)
                        ).into(),

                    ]).into(),
                    cosmic::widget::divider::horizontal::default().into(),

                    cosmic::widget::column::with_children(
                        tracks
                    ).into()
                ])
            )
        ).into()
    }
}

impl TrackPage {
    pub fn new() -> TrackPage {
        TrackPage {
            tracks: Arc::new(RwLock::from(vec![])),
            search: vec![],
            SearchTerm: String::from(""),
            track_page_state: TrackPageState::Loading,
            viewport: None,
            load_depth: 0,
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
            search_by_artist: false,
            search_by_album: false,
            search_by_title: false,
            size_opt: Cell::new(None),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.page()
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
            Message::PageDataRecieved(tracks)
        }).map(cosmic::Action::App);
    }
}

impl AppTrack {
    pub fn display(self) -> Element<'static, Message> {

            cosmic::widget::row::with_children(vec![
                widget::column![
                    cosmic::widget::text::caption_heading(self.title),
                    cosmic::widget::text::caption(self.artist),
                    cosmic::widget::text::caption(self.album_title),

                ].into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::button::text("x ^v >").into(),
            ])
                .height(Length::Fixed(64.0))
                .align_y(Vertical::Center)
                .into()
    }
}
