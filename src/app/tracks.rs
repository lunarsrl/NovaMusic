use crate::{app, fl};
use crate::app::{AppTrack, Message};
use cosmic::iced::{Alignment, Length};
use std::sync::Arc;
use cosmic::iced;
use cosmic::iced::widget::scrollable::Viewport;

#[derive(Debug, Clone)]
pub struct TrackPage {
    pub tracks: Arc<Vec<AppTrack>>,
    pub search: Vec<SearchResult>,
    pub track_page_state: TrackPageState,
    pub viewport: Option<Viewport>,
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
    pub fn new() -> Self {
        TrackPage {
            tracks: Arc::from(Vec::<AppTrack>::new()),
            search: vec![],
            track_page_state: TrackPageState::Loading,
            viewport: None,

            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
            search_by_artist: true,
            search_by_album: true,
            search_by_title: true,
        }
    }

    pub fn load_page<'a>(&'a self, model: &'a app::AppModel) -> cosmic::Element<'a, app::Message> {
        cosmic::widget::container::Container::new(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::container(
                cosmic::widget::row::with_children(vec![
                    // HEADING AREA
                    cosmic::widget::row::with_children(vec![
                        cosmic::widget::text::title2(fl!("TrackLibrary"))
                            .width(Length::FillPortion(2))
                            .into(),
                        cosmic::widget::horizontal_space()
                            .width(Length::Shrink)
                            .into(),
                        cosmic::widget::search_input(fl!("TrackInputPlaceholder"), &model.search_field)
                            .on_input(|a| Message::UpdateSearch(a))
                            .width(Length::FillPortion(1))
                            .into(),
                    ])
                    .align_y(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_s)
                    .into(),
                ])
                    .padding(cosmic::iced_core::Padding::from([
                        0,
                        cosmic::theme::spacing().space_m,
                    ]))
                )
                .into(),
                cosmic::widget::scrollable::vertical(
                    cosmic::widget::container(
                    match self.track_page_state {
                    TrackPageState::Loading => cosmic::widget::text::title3(fl!("Loading")).into(),
                    TrackPageState::Loaded => match self.tracks.is_empty() {
                        true => cosmic::widget::container(
                            cosmic::widget::column::with_children(vec![
                                cosmic::widget::text::title3("No Tracks Found In Database").into(),
                                cosmic::widget::text::text("1. Go to View -> Settings \n 2. Choose the directory where your music is located \n 3. Click on the red \"Rescan\" button to create your music database.").into(),
                                cosmic::widget::text::caption_heading("Cosmic Music currently support FLAC & MP3 files").into(),
                            ])
                                .spacing(cosmic::theme::spacing().space_s)
                        )
                            .align_x(Alignment::Center)

                            .width(Length::Fill)
                            .into(),

                        false => track_list_display(&self.tracks),
                    },
                    TrackPageState::Search => cosmic::widget::column::with_children(vec![
                        cosmic::widget::container(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading(fl!("SearchFilter")).into(),
                                cosmic::widget::horizontal_space().into(),
                                cosmic::widget::checkbox(fl!("title"), self.search_by_title)
                                    .on_toggle(|a| Message::ToggleTitle(a))
                                    .into(),
                                cosmic::widget::checkbox(fl!("album"), self.search_by_album)
                                    .on_toggle(|a| Message::ToggleAlbum(a))
                                    .into(),
                                cosmic::widget::checkbox(fl!("artist"), self.search_by_artist)
                                    .on_toggle(|a| Message::ToggleArtist(a))
                                    .into(),
                            ])
                            .spacing(cosmic::theme::spacing().space_s),
                        )
                        .padding(cosmic::theme::spacing().space_xxs)
                        .class(cosmic::style::Container::Primary)
                        .into(),
                        search_list_display(
                            &self.search,
                            &self.tracks,
                            (
                                self.search_by_title,
                                self.search_by_album,
                                self.search_by_artist,
                            ),
                        ),
                    ])
                    .spacing(cosmic::theme::spacing().space_m)
                    .into(),
                })
                        .padding(iced::core::padding::Padding::from([
                            0,
                            cosmic::theme::spacing().space_m,
                        ]))

                )
                .into(),
            ])
            .spacing(cosmic::theme::spacing().space_m),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn search_list_display<'a>(
    search_result: &'a Vec<SearchResult>,
    tracks: &'a Vec<AppTrack>,
    settings: (bool, bool, bool),
) -> cosmic::Element<'a, Message> {
    let mut title_vector: Vec<AppTrack> = vec![];
    let mut album_vector: Vec<AppTrack> = vec![];
    let mut artist_vector: Vec<AppTrack> = vec![];

    for each in search_result {
        if (0..=2).contains(&each.score) && settings.0 {
            match tracks.get(each.tracks_index) {
                None => {}
                Some(val) => {
                    title_vector.push(val.clone());
                }
            }
        } else if (3..=5).contains(&each.score) && settings.1 {
            match tracks.get(each.tracks_index) {
                None => {}
                Some(val) => {
                    album_vector.push(val.clone());
                }
            }
        } else if (6..=8).contains(&each.score) && settings.2 {
            match tracks.get(each.tracks_index) {
                None => {}
                Some(val) => {
                    artist_vector.push(val.clone());
                }
            }
        }
    }

    let mut elem_vec : Vec<cosmic::Element<Message>> = Vec::with_capacity(3);

    if settings.0 {
        elem_vec.push(search_group_display(&title_vector, fl!("title").as_str()));
    }

    if settings.1 {
        elem_vec.push(search_group_display(&album_vector, fl!("album").as_str()));
    }

    if settings.2 {
        elem_vec.push(search_group_display(&artist_vector, fl!("artist").as_str()));
    }


    cosmic::widget::column::with_children(
        elem_vec,
    )
        .spacing(cosmic::theme::spacing().space_s)
        .width(Length::Fill)
    .into()
}

fn search_group_display<'a>(tracks: &Vec<AppTrack>, search_title: &str) -> cosmic::Element<'a, Message> {
    cosmic::widget::column::with_children(vec![
        cosmic::widget::container(
            cosmic::widget::row::with_children(vec![
                cosmic::widget::text::heading(fl!("SearchFilterSpecify", filter = search_title)).into(),
            ])
                .padding(cosmic::theme::spacing().space_xxs)
                .width(Length::Fill)
        )
            .class(cosmic::theme::Container::Primary).into(),
        track_list_display(&tracks)
    ])
        .into()
}

fn track_list_display<'a>(tracks: &Vec<AppTrack>) -> cosmic::Element<'a, app::Message> {
    let mut list_widget = Some(cosmic::widget::ListColumn::new());

    for track in tracks {
        //todo if track is associated with an album, display album cover. Dont know how to do this efficiently yet.

        match list_widget.take() {
            Some(prev_list) => {
                list_widget = Some(
                    // ----CONTENT---- //
                    prev_list.add(cosmic::widget::container::Container::new(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(format!("{}", track.title,))
                                .width(Length::FillPortion(1))
                                .into(),
                            cosmic::widget::text::text(format!("{}", track.artist,))
                                .width(Length::FillPortion(1))
                                .into(),
                            cosmic::widget::text::text(format!("{}", track.album_title,))
                                .width(Length::FillPortion(1))
                                .into(),
                            cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                "media-playback-start-symbolic",
                            ))
                            .on_press(Message::AddTrackToQueue(
                                track.path_buf.to_string_lossy().to_string(),
                            ))
                            .into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_xxxs)
                        .align_y(Alignment::Center),
                    )),
                )
            }
            None => {}
        }
    }
    list_widget.unwrap().into_element()
}
