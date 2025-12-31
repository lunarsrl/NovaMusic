// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::PageBuilder;
use crate::app::{AppModel, AppTrack, Message};
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::Element;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct GenrePage {
    pub tracks: Arc<Vec<AppTrack>>,
    pub page_state: GenrePageState,
    pub has_fully_loaded: bool,
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced_core::widget::Id,
    pub search_term: String,
}

#[derive(Clone, Debug)]
pub enum GenrePageState {
    Loading,
    Search,
}

impl GenrePage {
    pub fn new() -> GenrePage {
        GenrePage {
            tracks: Arc::new(vec![]),
            page_state: GenrePageState::Loading,
            has_fully_loaded: false,
            viewport: None,
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
            search_term: "".to_string(),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.header()
    }
}
