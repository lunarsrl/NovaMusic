use crate::app::{AppTrack, Message};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::{iced, Element};
use std::sync::Arc;
use cosmic::widget::{Grid, JustifyContent, Widget};

#[derive(Debug, Clone)]
pub struct PlaylistPage {
    pub playlists: Arc<Vec<Playlist>>,
    pub playlist_page_state: PlaylistPageState,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub title: String,
    pub tracks: Vec<AppTrack>,
}

#[derive(Debug, Clone)]
pub enum PlaylistPageState {
    Loading,
    Loaded,
}

impl PlaylistPage {
    pub fn new() -> Self {
        PlaylistPage {
            playlists: Arc::new(vec![]),
            playlist_page_state: PlaylistPageState::Loaded,
        }
    }

    pub fn load_page(&self) -> Element<Message> {
        let body: Element<Message>;

        body = match self.playlist_page_state {
            PlaylistPageState::Loading => {
                cosmic::widget::container(cosmic::widget::text::title2("Loading..."))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .into()
            }
            PlaylistPageState::Loaded => {
                if !self.playlists.is_empty() {
                    return cosmic::widget::container(
                        cosmic::widget::column::with_children(
                            vec![
                                cosmic::widget::text::title3("No Playlists Found In Database").into(),
                                cosmic::widget::text::text("1. Add some music to your queue \n 2. Hit the \"Create Playlist\" button when you're ready \n 3. Enter some basic info and return to this page").into(),
                            ]
                        ).spacing(cosmic::theme::spacing().space_s)
                    )
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .into()
                } else {
                    let element : Vec<Element<Message>> = vec![];

                    let grid: Grid<Message> = cosmic::widget::Grid::new()
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .column_alignment(Alignment::Center)
                        .row_alignment(Alignment::Center)
                        .push(cosmic::widget::icon::from_name("media-optical-symbolic")
                                  .size(192),)
                        .push(cosmic::widget::icon::from_name("media-optical-symbolic")
                                  .size(192),)
                        .push(cosmic::widget::icon::from_name("media-optical-symbolic")
                                  .size(192),)
                        .push(cosmic::widget::icon::from_name("media-optical-symbolic")
                                  .size(192),)
                        .push(cosmic::widget::icon::from_name("media-optical-symbolic")
                                  .size(192),);



                    let val = cosmic::iced::widget::responsive(|size| {
                      cosmic::widget::container(

                          cosmic::widget::scrollable::vertical(
                          cosmic::widget::text::text(format!("size of container; y: {}, x: {}", size.height, size.width)),

                          )
                      )  .into()
                    });


                    val.into()
                }
            }
        };

        cosmic::widget::container(

            cosmic::widget::column::with_children(vec![
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text::title2("Playlists").into()
                ])
                .into(),
                body,
            ])
            .spacing(cosmic::theme::spacing().space_s),
        ).height(Length::Fill)
            .width(Length::Fill)
        .padding(iced::core::padding::Padding::from([
            0,
            cosmic::theme::spacing().space_m,
        ]))
        .into()
    }
}
