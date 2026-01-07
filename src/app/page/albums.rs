// SPDX-License-Identifier: GPL-2.0-or-later
use crate::app::page::tracks::SearchResult;
use crate::app::page::PageBuilder;
use crate::app::{AppModel, Message};
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::{Application, Element};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AlbumPage {
    pub albums: Arc<Vec<Album>>,
    pub page_state: AlbumPageState,
    pub has_fully_loaded: bool,
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced_core::widget::Id,
    pub search_term: String,
}

#[derive(Clone, Debug)]
pub enum AlbumPageState {
    /// Top level state, view of albums that have been loaded thus far
    Loading,
    /// Top level state, view once all items have been loaded, todo: for cache purposes eventually probably
    Loaded,
    /// State that shows view of all tracks of an album
    Album(FullAlbum),
    Search(Vec<SearchResult>),
}

impl AlbumPage {
    pub fn new() -> AlbumPage {
        AlbumPage {
            albums: Arc::new(vec![]),
            page_state: AlbumPageState::Loading,
            has_fully_loaded: false,
            viewport: None,
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
            search_term: "".to_string(),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.page()
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub(crate) disc_number: u32,
    pub(crate) track_number: u32,
    pub cover_art: Option<cosmic::widget::image::Handle>,
}

#[derive(Debug, Clone)]
pub struct FullAlbum {
    album: Album,
    tracks: Vec<Track>,
}

#[derive(Debug, Clone)]
struct Track {
    pub name: String,
    file_path: String,
    pub track_number: u32,
    disc_number: u32,
}
