// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::page::albums::{Album, FullAlbum};
use crate::app::page::tracks::SearchResult;
use crate::app::page::{BodyStyle, Page, PageBuilder};
use crate::app::AppModel;
use crate::app::{DisplaySingle, Message};
use crate::fl;
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{Alignment, Length};
use cosmic::Element;

#[derive(Clone, Debug)]
pub struct ArtistInfo {
    pub name: String,
    pub path: String,
    pub image: Option<cosmic::widget::image::Handle>,
}

#[derive(Debug)]
pub struct ArtistsPage {
    pub page_state: ArtistPageState,
    pub has_fully_loaded: bool,
    pub artists: Vec<ArtistInfo>,
    pub artist_page_cache: Option<ArtistPage>,

    //Scrollbar
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced::widget::Id,
    pub search_term: String,
}

#[derive(Debug)]
pub enum ArtistPageState {
    Loading,
    Loaded,
    ArtistPage(ArtistPage),
    Album(FullAlbum),
    Search(Vec<SearchResult>),
}

#[derive(Debug, Clone)]
pub struct ArtistPage {
    pub artist: ArtistInfo,
    pub singles: Vec<DisplaySingle>,
    pub albums: Vec<Album>,
}

impl ArtistsPage {
    pub fn new() -> ArtistsPage {
        ArtistsPage {
            page_state: ArtistPageState::Loading,
            has_fully_loaded: false,
            artists: vec![],
            artist_page_cache: None,
            viewport: None,
            scrollbar_id: cosmic::iced::widget::Id::unique(),
            search_term: String::from(""),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.page(model)
    }
}

impl Page for ArtistsPage {
    fn title(&self) -> String {
        String::from(fl!("artists"))
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        cosmic::widget::container(cosmic::widget::column::with_children(vec![
            cosmic::widget::indeterminate_circular().into(),
            cosmic::widget::indeterminate_linear().into(),
        ]))
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    fn body_style(&self) -> BodyStyle {
        return BodyStyle::List;
    }
}
