use crate::app::widget::scrollable::horizontal;
use crate::app::albums::FullAlbum;
use crate::app::{AppModel, AppTrack, Message};
use cosmic::iced::{Alignment, ContentFit, Length, Size};
use cosmic::widget::{Grid, JustifyContent, Widget};
use cosmic::{iced, Application, Element, Theme};
use std::path::PathBuf;
use std::sync::Arc;
use crate::app;
use crate::app::tracks::SearchResult;

#[derive(Debug, Clone)]
pub struct PlaylistPage {
    pub playlists: Arc<Vec<Playlist>>,
    pub playlist_page_state: PlaylistPageState,
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
    pub fn new() -> Self {
        PlaylistPage {
            playlists: Arc::new(vec![]),
            playlist_page_state: PlaylistPageState::Loading,
        }
    }

    pub fn load_page<'a>(&'a self, model: &'a AppModel) -> Element<'a, Message> {
        let body: Element<Message>;

        body = match &self.playlist_page_state {
            PlaylistPageState::Loading => {
                cosmic::widget::container(cosmic::widget::text::title2("Loading..."))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .into()
            }
            PlaylistPageState::Loaded => {
                if self.playlists.is_empty() {
                    return cosmic::widget::container(
                        cosmic::widget::column::with_children(
                            vec![
                                cosmic::widget::text::title3("No playlists found in playlists folder").into(),
                                cosmic::widget::text::text("1. Add some music to your queue \n 2. Hit the \"Create Playlist\" button when you're ready \n 3. Enter some basic info and return to this page").into(),
                                cosmic::widget::text::caption_heading(format!("Checking: {}", dirs::data_local_dir().unwrap().join(app::AppModel::APP_ID).join("Playlists").to_string_lossy().to_string())).into(),
                            ]
                        ).spacing(cosmic::theme::spacing().space_s)
                    )
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .into();
                } else {
                    cosmic::widget::container(cosmic::widget::responsive(move |size| {
                        // Body
                        let mut elements: Vec<Element<Message>> = vec![];

                        for playlist in self.playlists.as_ref() {
                            elements.push(
                                cosmic::widget::button::custom(
                                    cosmic::widget::column::with_children(vec![
                                        if let Some(cover_art) = &playlist.thumbnail {
                                            cosmic::widget::container::Container::new(
                                                cosmic::widget::image(cover_art),
                                            )
                                            .height((model.config.grid_item_size * 32) as f32)
                                            .width((model.config.grid_item_size * 32) as f32)
                                            .into()
                                        } else {
                                            cosmic::widget::container(
                                                cosmic::widget::icon::from_name(
                                                    "media-optical-symbolic",
                                                )
                                                .size((model.config.grid_item_size * 32) as u16),
                                            )
                                            .align_x(Alignment::Center)
                                            .align_y(Alignment::Center)
                                            .into()
                                        },
                                        cosmic::widget::column::with_children(vec![
                                            cosmic::widget::text::text(playlist.title.as_str())
                                                .center()
                                                .into(),
                                        ])
                                        .align_x(Alignment::Center)
                                        .width(cosmic::iced::Length::Fill)
                                        .into(),
                                    ])
                                    .align_x(Alignment::Center),
                                )
                                    .on_press(Message::PlaylistSelected(playlist.clone()))
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
                        let mut spacing: u16 = 0;
                        let mut items_per_row = 0;
                        let mut index = 0;

                        while width > (items_per_row * model.config.grid_item_size * 32) {
                            items_per_row += 1;
                        }
                        items_per_row -= 1;

                        let check_spacing: u32 =
                            ((items_per_row + 1) * model.config.grid_item_size * 32)
                                .saturating_sub(width);
                        let check_final = (model.config.grid_item_size * 32 - check_spacing);

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
            PlaylistPageState::PlaylistPage(playlist) => {
                return cosmic::widget::container(
                    // ALL
                    cosmic::widget::Column::with_children([
                        // HEADING
                        cosmic::widget::button::custom(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::icon::from_name("go-previous-symbolic").into(),
                                cosmic::widget::text::text("Playlists").into(),
                            ])
                            .align_y(Alignment::Center),
                        )
                        .class(cosmic::widget::button::ButtonClass::Link)
                            .on_press(Message::PlaylistPageReturn)
                        .into(),
                        cosmic::widget::Row::with_children([
                            match &playlist.playlist.thumbnail {
                                None => {
                                    cosmic::widget::icon::from_name("applications-audio-symbolic")
                                        .size(128)
                                        .into()
                                }
                                Some(handle) => cosmic::widget::image(handle)
                                    .content_fit(ContentFit::Contain)
                                    .height(128.0)
                                    .width(128.0)
                                    .into(),
                            },
                            cosmic::widget::Column::with_children([
                                // Album Title and Author Column
                                cosmic::widget::text::title2(playlist.playlist.title.as_str())
                                    .into(),
                                cosmic::widget::divider::horizontal::default().into(),
                                cosmic::widget::row::with_children(vec![

                                    cosmic::widget::button::text("Add to queue")
                                        .leading_icon(cosmic::widget::icon::from_name("media-playback-start-symbolic"))
                                        .class(cosmic::theme::Button::Suggested)
                                        .on_press(Message::AddAlbumToQueue(
                                            playlist
                                                .tracks
                                                .iter()
                                                .map(|a| a.path.clone())
                                                .collect::<Vec<String>>(),
                                        ))
                                        .into(),

                                    cosmic::widget::button::icon(cosmic::widget::icon::from_name("user-trash-symbolic"))
                                        .on_press(Message::PLaylistDeleteSafety)
                                        .class(cosmic::theme::Button::Destructive)
                                        .into(),

                                ])
                                    .spacing(cosmic::theme::spacing().space_s)
                                    .align_y(Alignment::Center)
                                    .into()

                            ])
                            .spacing(cosmic::theme::spacing().space_s)
                            .into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_s)
                        .into(),
                        // BODY
                        cosmic::widget::scrollable(cosmic::widget::container::Container::new(
                            tracks_listify(&playlist.tracks),
                        ))
                        .into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_m),
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                .padding(iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_m,
                ]))
                .into();
            },
            PlaylistPageState::Search(search_results) => {
                cosmic::widget::container(cosmic::widget::responsive(move |size| {
                    // Body
                    let mut elements: Vec<Element<Message>> = vec![];
                    let mut playlists: Vec<Playlist> = vec![];

                    for each in search_results {
                        if (0..=2).contains(&each.score) {
                            match self.playlists.get(each.tracks_index) {
                                None => {}
                                Some(val) => {
                                    playlists.push(val.clone());
                                }
                            }
                        }
                    }

                    for playlist in &playlists {
                        elements.push(
                            cosmic::widget::button::custom(
                                cosmic::widget::column::with_children(vec![
                                    if let Some(cover_art) = &playlist.thumbnail {
                                        cosmic::widget::container::Container::new(
                                            cosmic::widget::image(cover_art),
                                        )
                                            .height((model.config.grid_item_size * 32) as f32)
                                            .width((model.config.grid_item_size * 32) as f32)
                                            .into()
                                    } else {
                                        cosmic::widget::container(
                                            cosmic::widget::icon::from_name(
                                                "media-optical-symbolic",
                                            )
                                                .size((model.config.grid_item_size * 32) as u16),
                                        )
                                            .align_x(Alignment::Center)
                                            .align_y(Alignment::Center)
                                            .into()
                                    },
                                    cosmic::widget::column::with_children(vec![
                                        cosmic::widget::text::text(playlist.title.to_string())
                                            .center()
                                            .into(),
                                    ])
                                        .align_x(Alignment::Center)
                                        .width(cosmic::iced::Length::Fill)
                                        .into(),
                                ])
                                    .align_x(Alignment::Center),
                            )
                                .on_press(Message::PlaylistSelected(playlist.clone()))
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
                    let mut spacing: u16 = 0;
                    let mut items_per_row = 0;
                    let mut index = 0;

                    while width > (items_per_row * model.config.grid_item_size * 32) {
                        items_per_row += 1;
                    }
                    items_per_row -= 1;

                    let check_spacing: u32 =
                        ((items_per_row + 1) * model.config.grid_item_size * 32)
                            .saturating_sub(width);
                    let check_final = (model.config.grid_item_size * 32 - check_spacing);

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
        };

        cosmic::widget::container(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text::title2("Album Library")
                    .width(Length::FillPortion(2))
                    .into(),
                cosmic::widget::horizontal_space()
                    .width(Length::Shrink)
                    .into(),
                cosmic::widget::search_input(
                    "Enter Album Name",
                    model.search_field.as_str(),
                )
                    .on_input(|input| Message::UpdateSearch(input))
                    .width(Length::FillPortion(1))
                    .into(),
                ])
                    .align_y(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_s)
                    .into(),

                body,
            ])
            .spacing(cosmic::theme::spacing().space_s),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(iced::core::padding::Padding::from([
            0,
            cosmic::theme::spacing().space_m,
        ]))
        .into()
    }
}
fn tracks_listify(tracks: &Vec<PlaylistTrack>) -> Element<'static, Message> {
    let mut list_widget = Some(cosmic::widget::ListColumn::new());

    for track in tracks {
        match list_widget.take() {
            Some(prev_list) => {
                list_widget = Some(
                    // ----CONTENT---- //
                    prev_list.add(cosmic::widget::container::Container::new(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(format!("{}", track.title,)).into(),
                            cosmic::widget::horizontal_space().into(),
                            cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                "media-playback-start-symbolic",
                            ))
                            .on_press(Message::AddTrackToQueue(
                                track.path.clone(),
                            ))
                            .into(),
                        ])
                        .align_y(Alignment::Center),
                    )),
                )
            }

            None => {}
        }
    }
    list_widget.take().unwrap().into_element()
}
