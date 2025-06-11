use crate::app::Message;
use cosmic::cctk::wayland_client::backend::protocol::wl_message;
use cosmic::cosmic_theme::palette::cam16::Cam16IntoUnclamped;
use cosmic::widget::row;
use cosmic::{Apply, Element};
use rusqlite::Row;
use std::fmt::format;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct AlbumPage {
    albums: Option<Vec<Album>>,
    page_state: PageState,
}

#[derive(Clone, Debug)]
enum PageState {
    /// Top level state, view of albums that have been loaded thus far
    Loading,
    /// Top level state, view once all items have been loaded, todo: for cache purposes eventually probably
    // Loaded,
    /// State that shows view of all tracks of an album
    Album(FullAlbum),
}

impl AlbumPage {
    pub(crate) fn new(album_list: Option<Vec<Album>>) -> AlbumPage {
        AlbumPage {
            albums: album_list,
            page_state: PageState::Loading,
        }
    }

    pub fn new_album_page(album_info: FullAlbum) -> AlbumPage {
        AlbumPage {
            albums: None,
            page_state: PageState::Album(album_info),
        }
    }

    pub fn load_page(&self) -> Element<'static, Message> {
        let page_margin = cosmic::theme::spacing().space_m;
        match &self.page_state {
            PageState::Loading => {
                if self.albums.is_none() {
                    return cosmic::widget::text::title3("loading...").into();
                }

                let mut elements = vec![];

                for album in self.clone().albums.unwrap() {
                    elements.push(
                        cosmic::widget::button::custom(cosmic::widget::column::with_children(
                            vec![
                                cosmic::widget::icon::from_name("media-optical-symbolic").into(),
                                cosmic::widget::text::text(album.name.clone()).into(),
                                cosmic::widget::text::text(album.artist.clone()).into(),
                            ],
                        ))
                        .on_press(Message::AlbumRequested((album.name, album.artist)))
                        .width(192.0)
                        .height(192.0)
                        .into(),
                    )
                }
                cosmic::widget::container(cosmic::widget::flex_row(elements)).into()
            }
            PageState::Album(albumpage) => {
                cosmic::widget::container(
                    // ALL
                    cosmic::widget::Column::with_children([
                        // HEADING
                        cosmic::widget::Column::with_children([
                            cosmic::widget::text::title3(albumpage.album.name.clone()).into(),
                            cosmic::widget::text::title4(format!(
                                "By {}",
                                albumpage.album.artist.as_str()
                            ))
                            .into(),
                        ])
                        .into(),
                        // BODY
                        cosmic::widget::scrollable(
                            tracks_listify(&albumpage.tracks)
                        ).into()

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
    disc_number: u32,
    track_number: u32,
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
                    prev_list.add(
                         cosmic::widget::container::Container::new(
                             cosmic::widget::row::with_children(
                                 vec![
                                     cosmic::widget::text::heading(format!("{}. {}", track.track_number, track.name)).into(),
                                 ]
                             )
                         )
                    )
                )
            }
            None => {
                print!("idc")
            }
        }
    }

    list_widget.take().unwrap()
        .into_element()
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
    };

    let mut track_vector = vec![];

    let num_tracks = conn.query_row(
        "SELECT COUNT(*) FROM album_tracks where album_id = ?",
        [row_num.0.as_ref().unwrap_or(&0)],
        |row| row.get::<usize, u32>(0),
    );

    for each in 1..num_tracks.unwrap_or(0) {
        let tracks = conn
            .query_row(
                "SELECT * FROM album_tracks WHERE album_id = ? AND track_number = ?",
                [row_num.0.as_ref().unwrap_or(&0), &each],
                |row| {
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
                    Ok((track_dat, track_num, disc_num))
                },
            )
            .unwrap_or(((String::new(), String::new()), 0, 0));

        let track = Track {
            name: tracks.0 .0,
            file_path: tracks.0 .1,
            track_number: tracks.1,
            disc_number: tracks.2,
        };

        track_vector.push(track);
    }

    FullAlbum {
        album,
        tracks: track_vector,
    }
}

pub async fn get_top_album_info() -> Vec<Album> {
    let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();

    let row_num = conn
        .query_row(
            "SELECT COUNT(*) as row_count
    FROM album",
            (),
            |row| Ok(row.get::<usize, u32>(0).unwrap()),
        )
        .expect("error");

    let mut albums = Vec::new();

    for each in 1..=row_num {
        albums.push(
            match conn.query_row("SELECT * FROM album where id = ?", [each], |row| {
                let artists_id = row.get::<usize, i32>(2).unwrap();

                let artists_name = conn
                    .query_row("select * from artists where id = ?", [artists_id], |row| {
                        match row.get::<usize, String>(1) {
                            Ok(val) => Ok(val),
                            Err(_) => {
                                panic!("error")
                            }
                        }
                        //todo dont make the program crash if metadata is wrong
                    })
                    .unwrap_or_else(|_| "No Data".to_string());

                Ok(Album {
                    name: row.get::<usize, String>(1).unwrap(),
                    artist: artists_name,
                    disc_number: row.get::<usize, u32>(3).unwrap(),
                    track_number: row.get::<usize, u32>(4).unwrap(),
                })
            }) {
                Ok(val) => val,
                Err(_) => {
                    log::info!("EACH: {}", each);
                    panic!("error")
                }
            },
        );
    }

    albums
}
