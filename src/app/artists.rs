use crate::app::Message;
use cosmic::{iced, iced_core, Element};
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::widget::JustifyContent;
use iced::widget::scrollable::Viewport;
use crate::{app, fl};
use crate::app::{AppModel, AppTrack};
use crate::app::albums::Album;
use crate::app::tracks::SearchResult;

#[derive(Clone, Debug)]
pub struct ArtistInfo {
    pub name: String,
    pub image: Option<cosmic::widget::image::Handle>
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
pub enum ArtistPageState {
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
    pub fn load_page<'a>(&'a self, model: &'a AppModel) -> Element<'a, app::Message>{
        let body: Element<Message> = match &self.page_state {
            ArtistPageState::Loading => {
                cosmic::widget::text("LOADING").into()
            }
            ArtistPageState::Loaded => {
                if self.artists.is_empty() {
                    // todo Warning
                    cosmic::widget::text::text("no artists :(").into()
                } else {
                    cosmic::widget::container(cosmic::widget::responsive(move |size| {
                        // Body
                        let mut elements: Vec<Element<Message>> = vec![];

                        for artist in &self.artists {
                            elements.push(
                                cosmic::widget::button::custom(
                                    cosmic::widget::column::with_children(vec![
                                        if let Some(cover_art) = &artist.image {
                                            cosmic::widget::container::Container::new(
                                                cosmic::widget::image(cover_art).content_fit(ContentFit::Fill),
                                            )
                                                .height((model.config.grid_item_size * 32) as f32)
                                                .width((model.config.grid_item_size * 32) as f32)
                                                .into()
                                        } else {
                                            cosmic::widget::container(
                                                cosmic::widget::icon::from_name(
                                                    "playlist-symbolic",
                                                )
                                                    .size((model.config.grid_item_size * 32) as u16),
                                            )
                                                .align_x(Alignment::Center)
                                                .align_y(Alignment::Center)
                                                .into()
                                        },
                                        cosmic::widget::column::with_children(vec![
                                            cosmic::widget::text::text(artist.name.as_str())
                                                .center()
                                                .into(),
                                        ])
                                            .align_x(Alignment::Center)
                                            .width(cosmic::iced::Length::Fill)
                                            .into(),
                                    ])
                                        .align_x(Alignment::Center),
                                )

                                    .class(cosmic::widget::button::ButtonClass::Icon)
                                    .width((model.config.grid_item_size * 32) as f32)
                                    .into(),
                            )
                        }

                        let mut old_grid = Some(
                            cosmic::widget::Grid::new()
                                .width(Length::Fill)
                                .height(Length::Shrink),
                        );

                        let width = size.width as u32;
                        let spacing;
                        let mut items_per_row = 0;
                        let mut index = 0;

                        while width > (items_per_row * model.config.grid_item_size * 32) {
                            items_per_row += 1;
                        }
                        items_per_row -= 1;

                        let check_spacing: u32 =
                            ((items_per_row + 1) * model.config.grid_item_size * 32)
                                .saturating_sub(width);
                        let check_final = model.config.grid_item_size * 32 - check_spacing;

                        if items_per_row < 3 {
                            spacing = check_final as u16
                        } else {
                            spacing = (check_final / (items_per_row - 1)) as u16;
                        }

                        for element in elements {
                            index += 1;
                            if let Some(grid) = old_grid.take() {
                                if (index % items_per_row) == 0 {
                                    old_grid = Some(grid.push(element).insert_row());
                                } else {
                                    old_grid = Some(grid.push(element));
                                }
                            }
                        }

                        cosmic::widget::scrollable::vertical(
                            cosmic::widget::container(
                                old_grid
                                    .take()
                                    .unwrap()
                                    .column_spacing(spacing)
                                    .column_alignment(Alignment::Center)
                                    .justify_content(JustifyContent::Center)
                                    .row_alignment(Alignment::Center),
                            )
                                .align_x(Alignment::Center),
                        )
                            .into()
                    }))
                        .height(Length::Fill)
                        .into()
                }
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
                            fl!("ArtistInputPlaceholder"),
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
