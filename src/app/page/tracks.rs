// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::CoverArt::SomeLoaded;
use crate::app::page::{Page, PageBuilder};
use crate::app::{AppModel, AppTrack, Message};
use crate::fl;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{ContentFit, Length};
use cosmic::Element;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TrackPage {
    pub tracks: Arc<Vec<AppTrack>>,
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
        let mut tracks = vec![];
        for track in self.tracks.as_slice() {
            tracks.push(track.display())
        }

        cosmic::widget::scrollable(
            cosmic::widget::column::with_children(tracks), // .spacing(cosmic::theme::spacing().space_s)
        )
        .into()
    }
}

impl TrackPage {
    pub fn new() -> TrackPage {
        TrackPage {
            tracks: Arc::new(vec![]),
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
    pub fn display(&self) -> Element<Message> {
        let track_image = match &self.cover_art {
            SomeLoaded(visual) => cosmic::widget::image(visual)
                .width(32)
                .height(32)
                .content_fit(ContentFit::Cover)
                .into(),
            _ => cosmic::widget::icon::from_name("applications-multimedia-symbolic").into(),
        };
        cosmic::widget::container(
            cosmic::widget::row::with_children(vec![
                track_image,
                cosmic::widget::horizontal_space()
                    .width(Length::FillPortion(1))
                    .into(),
                cosmic::widget::text(self.title.as_str())
                    .width(Length::FillPortion(3))
                    .into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::text(self.artist.as_str())
                    .width(Length::FillPortion(3))
                    .into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::text(self.album_title.as_str())
                    .width(Length::FillPortion(3))
                    .into(),
            ])
            .align_y(Vertical::Center),
        )
        .into()
    }
}
