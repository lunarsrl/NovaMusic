// SPDX-License-Identifier: GPL-2.0-or-later
use crate::app::page::tracks::SearchResult;
use crate::app::page::BodyStyle::Grid;
use crate::app::page::{BodyStyle, CoverArt, Page, PageBuilder};
use crate::app::{connect_to_db, AppModel, AppTrack, Message};
use crate::fl;
use colored::Colorize;
use cosmic::iced::{Alignment, Color, ContentFit, Length};
use cosmic::iced_core::alignment::{Horizontal, Vertical};
use cosmic::iced_core::image::Handle;
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::iced_widget::text::Wrapping;
use cosmic::widget::settings::item;
use cosmic::widget::{icon, JustifyContent};
use cosmic::{iced_core, Element, Task};
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
    pub scrollbar_id: cosmic::iced_core::widget::Id,
    pub search_term: String,
}
const TextArea: f32 = 40.0;

impl Page for AlbumPage {
    fn title(&self) -> String {
        String::from(fl!("AlbumLibrary"))
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        let icon_size = model.config.grid_item_size;

        return cosmic::widget::container(cosmic::widget::responsive(move |size| {
            let width = size.width as u32;
            let spacing;
            let mut items_per_row = 0;
            let mut item_num = 0;

            while width > (items_per_row * icon_size * 32) {
                items_per_row += 1;
            }
            items_per_row -= 1;

            let check_spacing: u32 = ((items_per_row + 1) * icon_size * 32).saturating_sub(width);
            let check_final = icon_size * 32 - check_spacing;

            if items_per_row < 3 {
                spacing = check_final as u16
            } else {
                spacing = (check_final / (items_per_row - 1)) as u16;
            }

            let visible_rect = iced_core::Rectangle::new(
                iced_core::Point::new(
                    f32::from(cosmic::theme::spacing().space_s),
                    match self.viewport {
                        None => 0.0,
                        Some(val) => val.absolute_offset().y,
                    },
                ),
                iced_core::Size::new(3.0, size.height),
            );

            let mut album_rect = iced_core::Rectangle::new(
                iced_core::Point::new(f32::from(cosmic::theme::spacing().space_s), 0.0),
                iced_core::Size::new(3.0, icon_size as f32 * 32.0 + TextArea),
            );

            let mut grid = cosmic::widget::grid::<Message>()
                .column_spacing(spacing)
                .column_alignment(Alignment::Center)
                .justify_content(JustifyContent::Center)
                .row_alignment(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Shrink);

            for (index, album) in self.albums.clone().read().unwrap().iter().enumerate() {
                let insert_element;

                if album_rect.intersects(&visible_rect) {
                    insert_element = album.display_grid(icon_size);
                } else {
                    insert_element = cosmic::widget::column()
                        .push(cosmic::widget::text(format!("{}", index)))
                        .width(Length::Fill)
                        .height(Length::Fixed(icon_size as f32 * 32.0 + TextArea))
                        .into()
                }

                item_num += 1;

                if item_num as u32 % items_per_row == 0 {
                    log::info!(
                        "{}",
                        format!("new row {} --------\\", (index as f32 / 3.0).floor())
                            .to_string()
                            .red()
                    );
                    log::info!(
                        "visible area: startY: {} endY: {}",
                        visible_rect.y,
                        visible_rect.height + visible_rect.y
                    );
                    log::info!(
                        "album rect area: startY: {} endY {}",
                        album_rect.y,
                        album_rect.height + album_rect.y
                    );

                    grid = grid.push(insert_element).insert_row();
                    album_rect.y += icon_size as f32 * 32.0 + TextArea;
                } else {
                    grid = grid.push(insert_element);
                }
            }

            return cosmic::widget::scrollable::vertical(grid)
                .height(Length::Shrink)
                .on_scroll(|a| Message::ScrollView(a))
                .into();
        }))
        .height(Length::Fill)
        .into();
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

impl Album {
    fn display_grid<'a>(&self, size: u32) -> Element<'a, Message> {
        let art: Element<Message> = match &self.cover_art {
            None => cosmic::widget::icon::from_name("audio-x-generic")
                .size(size as u16 * 24)
                .into(),
            Some(art) => cosmic::widget::image(art)
                .content_fit(ContentFit::Contain)
                .width(Length::Fixed(size as f32 * 32.0))
                .height(Length::Fixed(size as f32 * 32.0))
                .into(),
        };

        return cosmic::widget::container(
            cosmic::widget::button::custom(
                cosmic::widget::column::with_children(vec![
                    art,
                    cosmic::widget::text::caption_heading(self.name.to_string()).into(),
                    cosmic::widget::text::caption(self.artist.to_string()).into(),
                ])
                .align_x(Horizontal::Center),
            )
            .on_press(Message::AlbumRequested((
                self.name.to_string(),
                self.artist.to_string(),
            )))
            .class(cosmic::theme::Button::MenuItem)
            .width(Length::Fixed(size as f32 * 32.0)),
        )
        .height(Length::Fixed(size as f32 * 32.0 + TextArea))
        .width(Length::Fixed(size as f32 * 32.0))
        .into();
    }
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
