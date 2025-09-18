use crate::app::Message;
use std::future::IntoFuture;
use std::sync::Arc;
use cosmic::{iced_core, Element};
use cosmic::iced::{Alignment, Length};
use cosmic::iced_renderer::graphics::Viewport;
use cosmic::iced_runtime::task::widget;
use cosmic::prelude::CollectionWidget;
use cosmic::widget::Widget;

use symphonia::core::conv::IntoSample;
use crate::{app, fl};
use crate::app::{AppModel, AppTrack};
use crate::app::albums::Album;
use crate::app::tracks::SearchResult;

#[derive(Clone, Debug)]
struct ArtistInfo {
    name: String,
    image: String,
}

#[derive(Debug)]
pub struct ArtistsPage {
    pub page_state: ArtistPageState,
    pub has_fully_loaded: bool,
    pub artists: Vec<ArtistInfo>,

    //Scrollbar
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced_core::widget::Id,
}

#[derive(Debug)]
enum ArtistPageState {
    Loading,
    Loaded,
    ArtistPage(ArtistPage),
    ArtistPageSearch(Vec<Vec<SearchResult>>),
    Search(Vec<SearchResult>),
}

#[derive(Debug)]
pub struct ArtistPage {
    singles: Vec<AppTrack>,
    albums: Vec<Album>
}

impl ArtistsPage {
    pub fn new() -> ArtistsPage {
        ArtistsPage {
            page_state: ArtistPageState::Loading,
            has_fully_loaded: false,
            artists: vec![],
            viewport: None,
            scrollbar_id: cosmic::iced_core::widget::Id::unique()
        }
    }
    pub fn load_page<'a>(&self, model: &'a AppModel) -> Element<'a, app::Message>{
        let body: Element<Message> = match &self.page_state {
            ArtistPageState::Loading => {
                cosmic::widget::text("LOADED").into()
            }
            ArtistPageState::Loaded => {
                cosmic::widget::text("LOADED").into()
            }
            ArtistPageState::ArtistPage(artistpage) => {
                // Unique State
                return cosmic::widget::container(cosmic::widget::text("Hello!")).into()
            }
            ArtistPageState::ArtistPageSearch(search) => {
                // Unique State
                return cosmic::widget::container(cosmic::widget::text("Hello!")).into()
            }
            ArtistPageState::Search(search) => {
                cosmic::widget::text("SEARCH").into()
            }
        };


        cosmic::widget::scrollable::vertical(
            cosmic::widget::container(
              cosmic::widget::column::with_children(vec![
                  // HEADING
                  cosmic::widget::row::with_children(vec![
                      cosmic::widget::text::title2(fl!("artists"))
                          .width(Length::FillPortion(2))
                          .into(),
                      cosmic::widget::horizontal_space()
                          .width(Length::Shrink)
                          .into(),
                      cosmic::widget::search_input(
                          fl!("PlaylistInputPlaceholder"),
                          model.search_field.as_str(),
                      )
                          .on_input(|input| Message::UpdateSearch(input))
                          .width(Length::FillPortion(1))
                          .into(),
                  ])
                      .align_y(Alignment::Center)
                      .spacing(cosmic::theme::spacing().space_s)
                      .into(),
                  // BODY
                  body,
              ])
            )
                .padding(iced_core::Padding::from([0, cosmic::theme::spacing().space_m]))

        )
            .into()
    }
}

