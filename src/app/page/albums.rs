mod fullalbum;

// SPDX-License-Identifier: GPL-2.0-or-later
use crate::app::page::style::GridPageStyle;
use crate::app::page::tracks::SearchResult;
use crate::app::page::BodyStyle::Grid;
use crate::app::page::{BodyStyle, CoverArt, Page, PageBuilder, PageType};
use crate::app::subpage::{Subpage, SubpageBuilder};
use crate::app::{connect_to_db, AppModel, AppTrack, Message};
use crate::fl;
use colored::Colorize;
use cosmic::iced::application::IntoBoot;
use cosmic::iced::core::text::EllipsizeHeightLimit;
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::widget::text::Ellipsize;
use cosmic::iced::{Alignment, Color, ContentFit, Length, Subscription};
use cosmic::widget::image::Handle;
use cosmic::widget::settings::item;
use cosmic::widget::{icon, JustifyContent};
use cosmic::{theme, Element, Task};
use rusqlite::ToSql;
use std::fmt::format;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct AlbumPage {
    pub albums: Arc<RwLock<Vec<Album>>>,
    pub page_state: AlbumPageState,
    pub has_fully_loaded: bool,
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced::widget::Id,
    pub search_term: String,
}
const TextArea: f32 = 40.0;

pub trait AlbumPageTrait {
    fn new_album_page();
    fn view();
}

impl AlbumPageTrait for PageType {
    fn new_album_page() {
        PageType {
            page_title: "".to_string(),
            data_stored: (),
            body_style: BodyStyle::Grid,
            scrollbar_id: Id(),
            viewport: ,
            size: Default::default(),
        };

    }

    fn view() {
        let scroll_id = cosmic::iced::widget::Id::unique();


        cosmic::widget::responsive(move |size| {
            cosmic::widget::id_container(, scroll_id).into()
        })
            .into();


    }
}
impl Page for AlbumPage {
    fn title(&self) -> String {
        String::from(fl!("AlbumLibrary"))
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        cosmic::widget::text::text("Hello!").into()
    }

    fn body_style(&self) -> BodyStyle {
        return BodyStyle::Grid;
    }
}

#[derive(Clone, Debug)]
pub enum AlbumPageState {
    Loading,
    Subpage(FullAlbum),
    Search(Vec<SearchResult>),
}

impl AlbumPage {
    pub fn new() -> AlbumPage {
        AlbumPage {
            albums: Arc::new(RwLock::new(vec![])),
            page_state: AlbumPageState::Loading,
            has_fully_loaded: false,
            viewport: None,
            scrollbar_id: cosmic::iced::widget::Id::unique(),
            search_term: "".to_string(),
        }
    }

    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        match &self.page_state {
            AlbumPageState::Subpage(album) => album.page(model),
            _ => cosmic::widget::responsive(move |size| {
                cosmic::widget::id_container(self.new_item_grid().view(), self.scrollbar_id.clone())
                    .into()
            })
            .into(),
        }
    }

    pub fn load_page_data(&self) -> Task<cosmic::Action<Message>> {
        return cosmic::Task::future(async move {
            let conn = connect_to_db();

            let mut stmt = conn
                .prepare(
                    "
                            select album.name as name, artists.name as aname, album.track_number as tn, album.disc_number as dn, album.album_cover as ac from album
                            left join main.artists artists on artists.id = album.artist_id
                    ",
                )
                .unwrap();

            let albums = stmt
                .query_map([], |row| {
                    Ok(Album {
                        name: row.get::<&str, String>("name").unwrap_or("N/A".to_string()),
                        artist: row.get::<&str, String>("aname").unwrap_or("N/A".to_string()),
                        disc_number: row.get::<&str, u32>("dn").unwrap_or(1),
                        track_number: row.get::<&str, u32>("tn").unwrap_or(1),
                        cover_art: match row.get::<&str, Vec<u8>>("ac") {
                            Ok(cover) => {
                               Some(cosmic::widget::image::Handle::from_bytes(cover))
                            },
                            Err(_) => {
                                None
                            }
                        } ,
                    })
                })
                .expect("Should never break");


            let albums = albums.filter_map(|a| a.ok()).collect::<Vec<Album>>();

            log::info!("Loading track data from the database done ");
            // log::info!("| time since entering the page {}ms", timer.elapsed().as_millis());
            Message::AlbumsDataRecieved(albums)
        })
            .map(cosmic::Action::App);
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub disc_number: u32,
    pub track_number: u32,
    pub cover_art: Option<cosmic::widget::image::Handle>,
}

#[derive(Debug, Clone)]
pub struct FullAlbum {
    pub album: Album,
    pub tracks: Vec<Track>,
}

impl FullAlbum {
    pub fn from_db(title: String, artist: String) -> FullAlbum {
        let conn = rusqlite::Connection::open(
            dirs::data_local_dir()
                .unwrap()
                .join("dev.lunarsrl.NovaMusic")
                .join("nova_music.db"),
        )
        .expect("Failed to create database connection");

        log::info!("{}", artist);
        let row_num;
        if artist.is_empty() || artist == "N/A" {
            row_num = conn
                .query_row(
                    "
SELECT * FROM album
WHERE album.name = ?
            ",
                    [title.as_str()],
                    |row| {
                        Ok((
                            row.get::<usize, u32>(0),
                            row.get::<&str, u32>("disc_number"),
                            row.get::<&str, u32>("track_number"),
                            row.get::<&str, Vec<u8>>("album_cover"),
                        ))
                    },
                )
                .unwrap();
        } else {
            row_num = conn
                .query_row(
                    "
                    SELECT * FROM album
                        left join artists art on album.artist_id = art.id
                    WHERE album.name = ? and art.name = ?
            ",
                    [title.as_str(), artist.as_str()],
                    |row| {
                        Ok((
                            row.get::<&str, u32>("id"),
                            row.get::<&str, u32>("disc_number"),
                            row.get::<&str, u32>("track_number"),
                            row.get::<&str, Vec<u8>>("album_cover"),
                        ))
                    },
                )
                .unwrap()
        }

        let album = Album {
            name: title,
            artist,
            disc_number: row_num.1.unwrap_or(1),
            track_number: row_num.2.unwrap_or(0),
            cover_art: match row_num.3 {
                Ok(bytes) => Some(cosmic::widget::image::Handle::from_bytes(bytes)),
                Err(_) => None,
            },
        };

        let mut track_vector = vec![];
        // Select all tracks with a certain album ID and count them
        let mut value = conn
            .prepare("select * from album_tracks where album_id = ?")
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
}

#[derive(Debug, Clone)]
struct Track {
    pub name: String,
    file_path: String,
    pub track_number: u32,
    disc_number: u32,
}
