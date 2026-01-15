// SPDX-License-Identifier: GPL-2.0-or-later
use crate::app::page::tracks::SearchResult;
use crate::app::page::{BodyStyle, CoverArt, Page, PageBuilder};
use crate::app::{connect_to_db, AppModel, AppTrack, Message};
use crate::fl;
use cosmic::iced::Length;
use cosmic::iced_core::alignment::Horizontal;
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::iced_widget::text::Wrapping;
use cosmic::{Element, Task};
use rusqlite::ToSql;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct AlbumPage {
    pub albums: Arc<RwLock<Vec<Album>>>,
    pub page_state: AlbumPageState,
    pub has_fully_loaded: bool,
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced_core::widget::Id,
    pub search_term: String,
}

impl Page for AlbumPage {
    fn title(&self) -> String {
        String::from(fl!("AlbumLibrary"))
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        let (width, height) = match self.viewport {
            None => (-1.0, -1.0),
            Some(val) => (val.bounds().width, val.bounds().height),
        };

        let mut grid_row: Vec<Element<Message>> = vec![];

        let test = self.albums.read().unwrap();

        if let Some(test) = test.get(9) {
            let art = test.cover_art.as_ref().unwrap();
            return cosmic::widget::container(
                cosmic::widget::column::with_children(vec![
                    cosmic::widget::image(art)
                        .width(Length::Fixed(model.config.grid_item_size as f32 * 32.0))
                        .height(Length::Fixed(model.config.grid_item_size as f32 * 32.0))
                        .into(),
                    cosmic::widget::text::text(test.name.to_string())..into(),
                    cosmic::widget::text::caption(test.artist.to_string()).into(),
                ])
                .align_x(Horizontal::Center):w,
            )
            .width(Length::Fixed(model.config.grid_item_size as f32 * 32.0))
            .into();
        } else {
            cosmic::widget::text("Loading...").into()
        }
    }

    fn body_style(&self) -> BodyStyle {
        return BodyStyle::Grid;
    }
}

#[derive(Clone, Debug)]
pub enum AlbumPageState {
    /// Top level state, view of albums that have been loaded thus far
    Loading,
    /// Top level state, view once all items have been loaded, todo: for cache purposes eventually probably
    Loaded,
    /// State that shows view of all tracks of an album
    Album(FullAlbum),
    Search(Vec<SearchResult>),
    Waiting,
}

impl AlbumPage {
    pub fn new() -> AlbumPage {
        AlbumPage {
            albums: Arc::new(RwLock::new(vec![])),
            page_state: AlbumPageState::Loading,
            has_fully_loaded: false,
            viewport: None,
            scrollbar_id: cosmic::iced_core::widget::Id::unique(),
            search_term: "".to_string(),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        self.page(model)
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
