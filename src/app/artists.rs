// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::albums::Album;
use crate::app::tracks::SearchResult;
use crate::app::AppModel;
use crate::app::{DisplaySingle, Message};
use crate::{app, fl};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::widget::{Dialog, JustifyContent};
use cosmic::{iced, Element};
use iced::widget::scrollable::Viewport;

#[derive(Clone, Debug)]
pub struct ArtistInfo {
    pub name: String,
    pub image: Option<cosmic::widget::image::Handle>,
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
            viewport: None,
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
        }
    }
    pub fn load_page<'a>(&'a self, model: &'a AppModel) -> Element<'a, app::Message> {
        let body: Element<Message> = match &self.page_state {
            ArtistPageState::Loading => cosmic::widget::text("LOADING").into(),
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
                                                cosmic::widget::image(cover_art)
                                                    .content_fit(ContentFit::Fill),
                                            )
                                            .height((model.config.grid_item_size * 32) as f32)
                                            .width((model.config.grid_item_size * 32) as f32)
                                            .into()
                                        } else {
                                            cosmic::widget::container(
                                                cosmic::widget::icon::from_name(
                                                    "avatar-default-symbolic",
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
                                .on_press(Message::ArtistRequested(artist.name.clone()))
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
                let data = artistpage.product_cover_button(model);

                return cosmic::widget::container(
                    cosmic::widget::column::with_children(vec![
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::button::custom(
                                cosmic::widget::row::with_children(vec![
                                    cosmic::widget::icon::from_name("go-previous-symbolic").into(),
                                    cosmic::widget::text::text(fl!("artists")).into(),
                                ])
                                .align_y(Alignment::Center),
                            )
                            .on_press(Message::ArtistPageReturn)
                            .class(cosmic::widget::button::ButtonClass::Link)
                            .into(),
                            cosmic::widget::horizontal_space().into(),
                            cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                "application-menu-symbolic",
                            ))
                            .class(cosmic::theme::Button::Standard)
                            .on_press(Message::ArtistPageEdit)
                            .into(),
                        ])
                        .align_y(Vertical::Center)
                        .into(),
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::icon::from_name("media-optical-symbolic")
                                .size(128)
                                .into(),
                            cosmic::widget::column::with_children(vec![
                                cosmic::widget::text::title3(artistpage.artist.name.as_str())
                                    .into(),
                                cosmic::widget::vertical_space().into(),
                                cosmic::widget::button::text(fl!("AddToQueue"))
                                    .leading_icon(cosmic::widget::icon::from_name(
                                        "media-playback-start-symbolic",
                                    ))
                                    .class(cosmic::theme::Button::Suggested)
                                    .into(),
                            ])
                            .height(Length::Fixed(128.0))
                            .spacing(cosmic::theme::spacing().space_s)
                            .into(),
                        ])
                        .align_y(Vertical::Center)
                        .spacing(cosmic::theme::spacing().space_s)
                        .into(),
                        cosmic::widget::divider::horizontal::default().into(),
                        // todo: let user customize whether an artist's singles appear in the grid or in the row.
                        // Maybe this can be done automatically by comparing the number of (tracks - albumtracks) to albumtracks but leave the user option
                        cosmic::widget::column::with_children(vec![
                            cosmic::widget::text::title4(fl!("albums")).into(),
                            cosmic::widget::scrollable::horizontal(
                                cosmic::widget::row::with_children(data.0).padding(
                                    cosmic::iced_core::padding::Padding::from([
                                        0,
                                        0,
                                        cosmic::theme::spacing().space_s,
                                        0,
                                    ]),
                                ),
                            )
                            .into(),
                            cosmic::widget::text::title4(fl!("tracks")).into(),
                            cosmic::widget::row::with_children(data.1).into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_m)
                        .into(),
                    ])
                    .padding(iced::core::padding::Padding::from([
                        0,
                        cosmic::theme::spacing().space_m,
                    ]))
                    .spacing(cosmic::theme::spacing().space_s),
                )
                .into();
            }
            ArtistPageState::ArtistPageSearch(search) => {
                // Unique State
                return cosmic::widget::container(cosmic::widget::text("Hello!")).into();
            }
            ArtistPageState::Search(search) => cosmic::widget::text("SEARCH").into(),
        };

        cosmic::widget::container(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text::title3(fl!("artists"))
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
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "application-menu-symbolic",
                    ))
                    .on_press(Message::ArtistsPageEdit)
                    .class(cosmic::widget::button::ButtonClass::Standard)
                    .into(),
                ])
                .align_y(Alignment::Center)
                .spacing(cosmic::theme::spacing().space_s)
                .into(),
                cosmic::widget::column::with_children(vec![cosmic::widget::row::with_children(
                    vec![cosmic::widget::horizontal_space().into()],
                )
                .spacing(cosmic::theme::spacing().space_xxs)
                .align_y(Vertical::Bottom)
                .into()])
                .spacing(cosmic::theme::spacing().space_xxs)
                .into(),
                cosmic::widget::divider::horizontal::default().into(),
                body,
            ])
            .spacing(cosmic::theme::spacing().space_xs),
        )
        .padding(iced::core::padding::Padding::from([
            0,
            cosmic::theme::spacing().space_m,
        ]))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    pub fn artist_edit_dialog(&self) -> Dialog<app::Message> {
        if let ArtistPageState::ArtistPage(page) = &self.page_state {
            let icon = match &page.artist.image {
                Some(handle) => cosmic::widget::container(
                    cosmic::widget::button::custom_image_button(
                        cosmic::widget::image(handle).content_fit(ContentFit::Fill),
                        None,
                    )
                    .on_press(Message::CreatePlaylistAddThumbnail),
                )
                .width(Length::Fixed(6.0 * 16.0))
                .height(Length::Fixed(6.0 * 16.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
                None => cosmic::widget::container(
                    cosmic::widget::button::icon(
                        cosmic::widget::icon::from_name("view-list-images-symbolic").size(6 * 8),
                    )
                    .padding(cosmic::theme::spacing().space_s)
                    .on_press(Message::CreatePlaylistAddThumbnail)
                    .class(cosmic::theme::Button::Suggested),
                )
                .class(cosmic::theme::Container::Secondary)
                .width(Length::Fixed(6.0 * 16.0))
                .height(Length::Fixed(6.0 * 16.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
            };

            cosmic::widget::dialog::Dialog::new()
                .title(format!("Edit {}'s Page", page.artist.name))
                .control(
                    cosmic::widget::container(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::horizontal_space().into(),
                            icon,
                            cosmic::widget::container(
                                cosmic::widget::column::with_children(vec![
                                    cosmic::widget::container(cosmic::widget::text(
                                        "Page Configuration",
                                    ))
                                    .padding(cosmic::theme::spacing().space_xxs)
                                    .into(),
                                    cosmic::widget::list::column::ListColumn::new()
                                        .add(
                                            cosmic::widget::row::with_children(vec![
                                                cosmic::widget::text::caption("Artist's Singles")
                                                    .into(),
                                                cosmic::widget::horizontal_space().into(),
                                                cosmic::widget::icon::from_name("go-down-symbolic")
                                                    .into(),
                                                cosmic::widget::icon::from_name("go-up-symbolic")
                                                    .into(),
                                            ])
                                            .spacing(cosmic::theme::spacing().space_xxs),
                                        )
                                        .add(
                                            cosmic::widget::row::with_children(vec![
                                                cosmic::widget::text::caption("Artist's Albums")
                                                    .into(),
                                                cosmic::widget::horizontal_space().into(),
                                                cosmic::widget::icon::from_name("go-down-symbolic")
                                                    .into(),
                                                cosmic::widget::icon::from_name("go-up-symbolic")
                                                    .into(),
                                            ])
                                            .spacing(cosmic::theme::spacing().space_xxs),
                                        )
                                        .into_element(),
                                ])
                                .align_x(Horizontal::Center),
                            )
                            .width(Length::Fixed(192.0))
                            .class(cosmic::style::Container::Secondary)
                            .into(),
                            cosmic::widget::horizontal_space().into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_s)
                        .align_y(Vertical::Center),
                    )
                    .align_x(Horizontal::Center),
                )
                .primary_action(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "object-select-symbolic",
                    ))
                    .class(cosmic::theme::Button::Suggested)
                    .on_press(Message::EditPlaylistConfirm),
                )
                .secondary_action(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "window-close-symbolic",
                    ))
                    .class(cosmic::theme::Button::Standard)
                    .on_press(Message::EditPlaylistCancel),
                )
        } else {
            panic!("Should always occur within ArtistPage ArtistPageState")
        }
    }

    // pub fn artists_edit_dialog<'a>() -> Dialog<'a, Element<'a, app::Message>>{
    //
    // }
}

impl ArtistPage {
    fn product_cover_button(
        &self,
        model: &AppModel,
    ) -> (Vec<Element<app::Message>>, Vec<Element<app::Message>>) {
        let mut singles = vec![];
        let mut albums = vec![];

        for single in self.singles.as_slice() {
            singles.push(
                cosmic::widget::button::custom(cosmic::widget::column::with_children(vec![
                    if let Some(cover_art) = &single.cover_art {
                        cosmic::widget::container::Container::new(
                            cosmic::widget::image(cover_art).content_fit(ContentFit::Fill),
                        )
                        .height((model.config.grid_item_size * 32) as f32)
                        .width((model.config.grid_item_size * 32) as f32)
                        .into()
                    } else {
                        cosmic::widget::container(
                            cosmic::widget::icon::from_name("media-optical-symbolic").size(192),
                        )
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .into()
                    },
                    cosmic::widget::column::with_children(vec![
                        cosmic::widget::text::text(single.title.as_str())
                            .center()
                            .into(),
                        cosmic::widget::text::text(single.artist.as_str())
                            .center()
                            .into(),
                    ])
                    .align_x(Alignment::Center)
                    .width(cosmic::iced::Length::Fill)
                    .into(),
                ]))
                .class(cosmic::widget::button::ButtonClass::Icon)
                .on_press(Message::AddTrackById((single.id)))
                .width((model.config.grid_item_size * 32) as f32)
                .into(),
            )
        }

        for album in self.albums.as_slice() {
            albums.push(
                cosmic::widget::button::custom(cosmic::widget::column::with_children(vec![
                    if let Some(cover_art) = &album.cover_art {
                        cosmic::widget::container::Container::new(
                            cosmic::widget::image(cover_art).content_fit(ContentFit::Fill),
                        )
                        .height((model.config.grid_item_size * 32) as f32)
                        .width((model.config.grid_item_size * 32) as f32)
                        .into()
                    } else {
                        cosmic::widget::container(
                            cosmic::widget::icon::from_name("media-optical-symbolic").size(192),
                        )
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .into()
                    },
                    cosmic::widget::column::with_children(vec![
                        cosmic::widget::text::text(album.name.as_str())
                            .center()
                            .into(),
                        cosmic::widget::text::text(album.artist.as_str())
                            .center()
                            .into(),
                    ])
                    .align_x(Alignment::Center)
                    .width(cosmic::iced::Length::Fill)
                    .into(),
                ]))
                .class(cosmic::widget::button::ButtonClass::Icon)
                .on_press(Message::AlbumRequested((
                    album.name.clone(),
                    album.artist.clone(),
                )))
                .width((model.config.grid_item_size * 32) as f32)
                .into(),
            )
        }

        (albums, singles)
    }
}
