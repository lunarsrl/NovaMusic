use crate::app;
use crate::app::Message;
use cosmic::cctk::wayland_client::backend::protocol::wl_message;
use cosmic::cosmic_theme::palette::blend::Compose;
use cosmic::cosmic_theme::palette::cam16::Cam16IntoUnclamped;
use cosmic::iced::futures::channel::mpsc::Sender;
use cosmic::iced::{Alignment, Length, Size};
use cosmic::widget::image::Handle::Bytes;
use cosmic::widget::row;
use cosmic::{iced, iced_core, Apply, Element};
use futures_util::SinkExt;
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::{params, MappedRows, Row};
use std::fmt::format;
use std::time;
use std::time::{Duration, SystemTime};
use cosmic::iced::wgpu::naga::back::spv::Capability::MeshShadingEXT;

#[derive(Clone, Debug)]
pub struct AlbumPage {
    pub albums: Option<Vec<Album>>,
    pub page_state: PageState,
    pub has_fully_loaded: bool,
}

#[derive(Clone, Debug)]
pub enum PageState {
    /// Top level state, view of albums that have been loaded thus far
    Loading,
    /// Top level state, view once all items have been loaded, todo: for cache purposes eventually probably
    Loaded,
    /// State that shows view of all tracks of an album
    Album(FullAlbum),
}

impl AlbumPage {
    pub(crate) fn new(album_list: Option<Vec<Album>>) -> AlbumPage {
        AlbumPage {
            albums: album_list,
            page_state: PageState::Loading,
            has_fully_loaded: false,
        }
    }

    pub fn load_page(&self) -> Element<'static, Message> {
        let page_margin = cosmic::theme::spacing().space_m;
        match &self.page_state {
            PageState::Loading | PageState::Loaded => {
                if self.albums.is_none() {
                    if let PageState::Loading = self.page_state {

                        return cosmic::widget::container(cosmic::widget::text::title3("Loading..."))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into();
                    } else {
                        return cosmic::widget::container(
                            cosmic::widget::column::with_children(
                                vec![
                                    cosmic::widget::text::title3("No Albums Found In Database").into(),
                                    cosmic::widget::text::text("1. Go to View -> Settings \n 2. Choose the directory where your music is located \n 3. Click on the red \"Rescan\" button to create your music database.").into(),
                                    cosmic::widget::text::caption_heading("If the issue persists, your files may lack the metadata to be identified as albums. A tool like MusicBrainz Picard can help you add and organize music metadata.").into(),
                                ]
                            ).spacing(cosmic::theme::spacing().space_s)
                        )
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into();
                    }

                    
                }
                let mut elements = vec![];
                for album in self.clone().albums.unwrap() {
                    elements.push(
                        cosmic::widget::button::custom(cosmic::widget::column::with_children(
                            // Need to turn this into an async task, just wanted to see what it would look like for now
                            vec![
                                if let Some(cover_art) = &album.cover_art {
                                    cosmic::widget::container::Container::new(
                                        cosmic::widget::image(cover_art),
                                    )
                                    .align_y(Alignment::Center)
                                    .align_x(Alignment::Center)
                                    .height(192.0)
                                    .width(192.0)
                                    .into()
                                } else {
                                    cosmic::widget::container(
                                        cosmic::widget::icon::from_name("media-optical-symbolic")
                                            .size(192),
                                    )
                                    .align_x(Alignment::Center)
                                    .align_y(Alignment::Center)
                                    .into()
                                },
                                cosmic::widget::column::with_children(vec![
                                    cosmic::widget::text::text(album.name.clone()).into(),
                                    cosmic::widget::text::text(album.artist.clone()).into(),
                                ])
                                .align_x(Alignment::Center)
                                .width(cosmic::iced::Length::Fill)
                                .into(),
                            ],
                        ))
                        .class(cosmic::widget::button::ButtonClass::Icon)
                        .on_press(Message::AlbumRequested((album.name, album.artist)))
                        .width(192.0)
                        .into(),
                    )
                }

                cosmic::widget::scrollable(cosmic::widget::container(cosmic::widget::flex_row(
                    elements,
                )))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
            PageState::Album(albumpage) => {
                cosmic::widget::container(
                    // ALL
                    cosmic::widget::Column::with_children([
                        // HEADING
                        cosmic::widget::Row::with_children([
                            cosmic::widget::Column::with_children([
                                cosmic::widget::button::custom(
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::icon::from_name("go-previous-symbolic")
                                            .into(),
                                        cosmic::widget::text::text("Albums").into(),
                                    ])
                                    .align_y(Alignment::Center),
                                )
                                .class(cosmic::widget::button::ButtonClass::Link)
                                .on_press(Message::AlbumPageReturn)
                                .into(),
                                cosmic::widget::text::title3(albumpage.album.name.clone()).into(),
                                cosmic::widget::text::title4(format!(
                                    "By {}",
                                    albumpage.album.artist.as_str()
                                ))
                                .into(),
                            ])
                            .into(),
                        ])
                        .into(),
                        // BODY
                        cosmic::widget::scrollable(cosmic::widget::container::Container::new(
                            tracks_listify(&albumpage.tracks),
                        ))
                        .into(),
                    ])
                    .spacing(page_margin),
                )
                .padding(page_margin)
                .into()
            }
        }
    }

    pub fn modify_page_state(self) {}
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

fn tracks_listify(tracks: &Vec<Track>) -> Element<'static, Message> {
    let mut list_widget = Some(cosmic::widget::ListColumn::new());

    for track in tracks {
        match list_widget.take() {
            Some(prev_list) => {
                list_widget = Some(
                    // ----CONTENT---- //
                    prev_list.add(cosmic::widget::container::Container::new(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(
                            format!("{}. {}", track.track_number, track.name),
                        )
                        .into(),
                            cosmic::widget::container(
                            cosmic::widget::button::custom(
                                cosmic::widget::row::with_children(vec![
                                    cosmic::widget::icon::from_name("media-playback-start-symbolic")
                                        .into(),
                                    cosmic::widget::text::text("Play!").into(),
                                ])
                                    .align_y(Alignment::Center),
                            )
                                .on_press(Message::StartStream(track.file_path.parse().unwrap()))
                                .class(cosmic::widget::button::ButtonClass::Link)
                            )
                                .align_x(Alignment::End)
                                .into()
                        ]),
                    )),
                )
            }
            None => {
                print!("idc")
            }
        }
    }

    list_widget.take().unwrap().into_element()
}

impl Track {
    fn track_list_itemify(&self) -> Element<'static, Message> {
        cosmic::widget::text::heading(format!(
            "{}. {}",
            self.track_number.to_string(),
            self.name.to_string()
        ))
        .into()
    }
}

pub async fn get_album_info(title: String, artist: String) -> FullAlbum {
    let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();
    let row_num = conn
        .query_row(
            "SELECT * FROM album WHERE name = ?",
            [title.as_str()],
            |row| {
                Ok((
                    row.get::<usize, u32>(0),
                    row.get::<usize, u32>(3),
                    row.get::<usize, u32>(4),
                ))
            },
        )
        .unwrap();

    let album = Album {
        name: title,
        artist,
        disc_number: row_num.1.unwrap_or(0),
        track_number: row_num.2.unwrap_or(0),
        cover_art: None,
    };

    let mut track_vector = vec![];

    // Select all tracks with a certain album ID and count them
    let num_tracks = conn.query_row(
        "SELECT COUNT(*) FROM album_tracks where album_id = ?",
        [row_num.0.as_ref().unwrap_or(&0)],
        |row| row.get::<usize, u32>(0),
    );

    let mut value = conn
        .prepare("SELECT * FROM album_tracks where album_id = ?")
        .expect("error preparing sql to fetch album tracks of a certain album id");
    let mut rows = value
        .query([row_num.0.expect("No row num, shouldve exited by now")])
        .expect("error fetching album tracks of a certain album id");

    while let Some(row) = rows.next().unwrap() {
        let track_num = row.get::<usize, u32>(3).unwrap();
        let disc_num = row.get::<usize, u32>(4).unwrap();

        let track_dat = match row.get::<usize, u32>(2) {
            Ok(val) => conn
                .query_row("SELECT name, path FROM track WHERE id = ?", [val], |row| {
                    Ok((
                        row.get::<usize, String>(0)
                            .unwrap_or(String::from("NOT FOUND")),
                        row.get::<usize, String>(1)
                            .unwrap_or(String::from("NOT FOUND")),
                    ))
                })
                .unwrap_or((String::from("ERROR"), String::from("ERROR"))),
            Err(_) => {
                panic!("NO ID")
            }
        };

        let track = Track {
            name: track_dat.0,
            file_path: track_dat.1,
            track_number: track_num,
            disc_number: disc_num,
        };
        track_vector.push(track);
        track_vector.sort_by(|a, b| a.track_number.cmp(&b.track_number))
    }

    FullAlbum {
        album,
        tracks: track_vector,
    }
}

pub async fn get_top_album_info(
    tx: &mut Sender<Message>,
    album_iter: Vec<(String, String, u32, u32, Option<Vec<u8>>)>,
) {
    // Prepare and execute query in a separate scope
    // Process results and send them
    for album_result in album_iter {
        let album = Album {
            name: album_result.0,
            artist: album_result.1,
            disc_number: album_result.2,
            track_number: album_result.3,
            cover_art: match album_result.4 {
                None => None,
                Some(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
            },
        };

        tx.send(Message::AlbumProcessed(album))
            .await
            .expect("Failed to send album");
    }
}
