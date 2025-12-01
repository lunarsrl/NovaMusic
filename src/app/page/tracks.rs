// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::PageBuilder;
use crate::app::{AppModel, AppTrack, Message};
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::ContentFit;
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
        self.header(model.search_field.clone())
    }
}

impl AppTrack {
    fn display(&self) -> Element<Message> {
        let track_image = match &self.cover_art {
            None => cosmic::widget::icon::from_name("store-relax-symbolic")
                .size(32)
                .into(),
            Some(cover) => cosmic::widget::image(cover)
                .width(32)
                .height(32)
                .content_fit(ContentFit::Cover)
                .into(),
        };
        cosmic::widget::container(cosmic::widget::row::with_children(vec![
            track_image,
            cosmic::widget::text(self.title.as_str()).into(),
            cosmic::widget::text(self.artist.as_str()).into(),
            cosmic::widget::text(self.album_title.as_str()).into(),
        ]))
        .into()
    }
}
