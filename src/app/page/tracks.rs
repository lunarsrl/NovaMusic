// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::CoverArt::SomeLoaded;
use crate::app::page::{Page, PageBuilder};
use crate::app::{AppModel, AppTrack, Message};
use crate::fl;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{ContentFit, Length};
use cosmic::Element;
use std::sync::{Arc, Mutex, RwLock};
use rayon::iter::IntoParallelIterator;

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
}

#[derive(Debug, Clone)]
pub enum TrackPageState {
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
        log::info!("LOCK STATE: {:?}", self.tracks.is_poisoned());
        let mut tracks = vec![];

        for (index, track) in self.tracks.clone().read().unwrap().iter().enumerate() {
            let owned_track = track.clone();
            let display_element: Element<Message> = owned_track.display().into();

            if index % 2 == 0 {
                tracks.push(
                    cosmic::widget::container::Container::new(display_element).class(cosmic::theme::Container::Primary).into()
                )
            } else {
                tracks.push(
                    cosmic::widget::container::Container::new(display_element).class(cosmic::theme::Container::List).into()
                )
            }
        }

        return cosmic::widget::scrollable(
            cosmic::widget::column::with_children(tracks), // .spacing(cosmic::theme::spacing().space_s)
        )
        .into()
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
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.page()
    }
}

impl AppTrack {
    pub fn display(self) -> Element<'static, Message> {
        let track_image = match &self.cover_art {
            SomeLoaded(visual) => {
                cosmic::widget::image(visual)
                .width(64)
                .height(64)
                .into()},
            _ => cosmic::widget::icon::from_name("applications-multimedia-symbolic").into(),
        };

            cosmic::widget::row::with_children(vec![
                track_image,
                cosmic::widget::horizontal_space()
                    .width(Length::FillPortion(1))
                    .into(),
                cosmic::widget::text(self.title.to_string())
                    .width(Length::FillPortion(3))
                    .into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::text(self.artist.to_string())
                    .width(Length::FillPortion(3))
                    .into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::text(self.album_title.to_string())
                    .width(Length::FillPortion(3))
                    .into(),
            ])
                .height(Length::Fixed(64.0))
                .align_y(Vertical::Center)
                .into()
    }
}
