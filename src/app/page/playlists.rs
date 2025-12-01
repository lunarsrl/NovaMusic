// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::tracks::SearchResult;
use crate::app::page::PageBuilder;
use crate::app::{AppModel, Message};
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::{Application, Element};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PlaylistPage {
    pub viewport: Option<Viewport>,
    pub playlists: Arc<Vec<Playlist>>,
    pub playlist_page_state: PlaylistPageState,
    pub scrollbar_id: cosmic::iced_core::widget::Id,
    pub search_term: String,
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub title: String,
    pub path: String,
    pub thumbnail: Option<cosmic::widget::image::Handle>,
}
#[derive(Debug, Clone)]
pub struct FullPlaylist {
    pub playlist: Playlist,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Debug, Clone)]
pub struct PlaylistTrack {
    pub(crate) title: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone)]
pub enum PlaylistPageState {
    Loading,
    Loaded,
    PlaylistPage(FullPlaylist),
    Search(Vec<SearchResult>),
}

impl PlaylistPage {
    pub fn new() -> PlaylistPage {
        PlaylistPage {
            playlists: Arc::new(vec![]),
            playlist_page_state: PlaylistPageState::Loading,
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
            viewport: None,
            search_term: "".to_string(),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.header(model.search_field.clone())
    }
}
