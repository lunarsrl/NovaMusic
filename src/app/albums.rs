use crate::app::tracks::SearchResult;
use crate::app::{AppModel, Message};
use colored::Colorize;
use cosmic::iced::futures::channel::mpsc::Sender;
use cosmic::iced::{Alignment, ContentFit, Length, Size};
use cosmic::widget::settings::item;
use cosmic::widget::{container, JustifyContent};
use cosmic::{iced, widget, Application, Element, Theme};
use futures_util::{SinkExt, StreamExt};
use rusqlite::fallible_iterator::FallibleIterator;
use std::sync::Arc;
use cosmic::iced_widget::Container;

#[derive(Clone, Debug)]
pub struct AlbumPage {
    pub albums: Arc<Vec<Album>>,
    pub page_state: AlbumPageState,
    pub has_fully_loaded: bool,
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
}

impl AlbumPage {
    pub(crate) fn new(album_list: Vec<Album>) -> AlbumPage {
        AlbumPage {
            albums: Arc::from(album_list),
            page_state: AlbumPageState::Loading,
            has_fully_loaded: false,
        }
    }

    pub fn load_page<'a>(&'a self, model: &'a AppModel) -> Element<'a, Message> {
        let page_margin = cosmic::theme::spacing().space_m;
        match &self.page_state {
            AlbumPageState::Loading | AlbumPageState::Loaded => {
                if self.albums.is_empty() {
                    return if let AlbumPageState::Loading = self.page_state {
                        cosmic::widget::container(cosmic::widget::text::title3("Loading..."))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    } else {
                        cosmic::widget::container(

                            cosmic::widget::column::with_children(
                                vec![

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
                                        .padding(iced::core::padding::Padding::from([
                                            0,
                                            cosmic::theme::spacing().space_m,
                                        ]))
                                        .align_y(Alignment::Center)
                                        .spacing(cosmic::theme::spacing().space_s)
                                        .into(),
                                    cosmic::widget::text::title3("No Albums Found In Database").into(),
                                    cosmic::widget::text::text("1. Go to View -> Settings \n 2. Choose the directory where your music is located \n 3. Click on the red \"Rescan\" button to create your music database.").into(),
                                    cosmic::widget::text::caption_heading("If the issue persists, your files may lack the metadata to be identified as albums. A tool like MusicBrainz Picard or Kid3 can help you add and organize music metadata.").into(),
                                ]
                            )
                                .spacing(cosmic::theme::spacing().space_s)
                        )

                            .padding(cosmic::theme::spacing().space_xxs)
                            .align_x(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    };
                }

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
                        .padding(iced::core::padding::Padding::from([
                            0,
                            cosmic::theme::spacing().space_m,
                        ]))
                        .align_y(Alignment::Center)
                        .spacing(cosmic::theme::spacing().space_s)
                        .into(),
                        cosmic::widget::container(cosmic::widget::responsive(move |size| {
                            // Body
                            let mut elements: Vec<Element<Message>> = vec![];

                            for album in self.albums.as_ref() {
                                elements.push(
                                    cosmic::widget::button::custom(
                                        cosmic::widget::column::with_children(vec![
                                            if let Some(cover_art) = &album.cover_art {
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
                                                    .size(192),
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
                                        ]),
                                    )
                                    .class(cosmic::widget::button::ButtonClass::Icon)
                                    .on_press(Message::AlbumRequested((
                                        album.name.clone(),
                                        album.artist.clone(),
                                    )))
                                    .width((model.config.grid_item_size * 32) as f32)
                                    .into(),
                                )
                            }

                            let mut old_grid = Some(
                                cosmic::widget::Grid::new()
                                    .width(Length::Fill)
                                    .height(Length::Shrink),
                            );

                            let width =
                                size.width as u32 - cosmic::theme::spacing().space_m as u32 * 2;
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
                        .into(),
                    ])
                    .align_x(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_s),
                )
                .align_x(Alignment::Center)
                .into()
            }
            AlbumPageState::Album(albumpage) => {
                cosmic::widget::container(
                    // ALL
                    cosmic::widget::Column::with_children([
                        // HEADING
                        cosmic::widget::button::custom(
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::icon::from_name("go-previous-symbolic").into(),
                                cosmic::widget::text::text("Albums").into(),
                            ])
                            .align_y(Alignment::Center),
                        )
                        .class(cosmic::widget::button::ButtonClass::Link)
                        .on_press(Message::AlbumPageReturn)
                        .into(),
                        cosmic::widget::Row::with_children([
                            // Art Area?
                            match &albumpage.album.cover_art {
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
                                cosmic::widget::text::title2(albumpage.album.name.clone()).into(),
                                cosmic::widget::text::title4(format!(
                                    "By {}",
                                    albumpage.album.artist.as_str()
                                ))
                                .into(),
                                cosmic::widget::button::custom(
                                    cosmic::widget::row::with_children(vec![
                                        cosmic::widget::icon::from_name(
                                            "media-playback-start-symbolic",
                                        )
                                        .into(),
                                        cosmic::widget::text::text("Add Album To Queue").into(),
                                    ])
                                    .spacing(cosmic::theme::spacing().space_xxs)
                                    .align_y(Alignment::Center),
                                )
                                .padding(cosmic::theme::spacing().space_xxs)
                                .on_press(Message::AddAlbumToQueue(
                                    albumpage
                                        .tracks
                                        .iter()
                                        .map(|a| a.file_path.clone())
                                        .collect::<Vec<String>>(),
                                ))
                                .class(cosmic::widget::button::ButtonClass::Suggested)
                                .into(),
                            ])
                            .spacing(cosmic::theme::spacing().space_xxxs)
                            .into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_s)
                        .into(),
                        // BODY
                        cosmic::widget::scrollable(cosmic::widget::container::Container::new(
                            tracks_listify(&albumpage.tracks, albumpage.album.disc_number),
                        ))
                        .into(),
                    ])
                    .spacing(page_margin),
                )
                .padding(iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_m,
                ]))
                .into()
            }
            AlbumPageState::Search(search_results) => {
                cosmic::widget::container(cosmic::widget::column::with_children(vec![
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
                    .padding(iced::core::padding::Padding::from([
                        0,
                        cosmic::theme::spacing().space_m,
                    ]))
                    .align_y(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_s)
                    .into(),
                    cosmic::widget::container(cosmic::widget::responsive(move |size| {
                        // Body
                        let mut elements: Vec<Element<Message>> = vec![];
                        let mut albums: Vec<Album> = vec![];

                        for each in search_results {
                            if (0..=2).contains(&each.score) {
                                match self.albums.get(each.tracks_index) {
                                    None => {}
                                    Some(val) => {
                                        albums.push(val.clone());
                                    }
                                }
                            }
                        }

                        for album in albums {
                            elements.push(
                                cosmic::widget::button::custom(
                                    cosmic::widget::column::with_children(vec![
                                        if let Some(cover_art) = &album.cover_art {
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
                                                .size(192),
                                            )
                                            .align_x(Alignment::Center)
                                            .align_y(Alignment::Center)
                                            .into()
                                        },
                                        cosmic::widget::column::with_children(vec![
                                            cosmic::widget::text::text(album.name.clone())
                                                .center()
                                                .into(),
                                            cosmic::widget::text::text(album.artist.clone())
                                                .center()
                                                .into(),
                                        ])
                                        .align_x(Alignment::Center)
                                        .width(cosmic::iced::Length::Fill)
                                        .into(),
                                    ]),
                                )
                                .class(cosmic::widget::button::ButtonClass::Icon)
                                .on_press(Message::AlbumRequested((
                                    album.name.clone(),
                                    album.artist.clone(),
                                )))
                                .width((model.config.grid_item_size * 32) as f32)
                                .into(),
                            )
                        }

                        let mut old_grid = Some(
                            cosmic::widget::Grid::new()
                                .width(Length::Fill)
                                .height(Length::Shrink),
                        );

                        let width = size.width as u32 - cosmic::theme::spacing().space_m as u32 * 2;
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
                    .into(),
                ]))
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

fn tracks_listify<'a>(tracks: &Vec<Track>, num_of_discs: u32) -> Element<'a, Message> {
    log::info!("Number of discs: {}", num_of_discs);
    let mut discs: Vec< Vec<cosmic::widget::Container<Message, Theme> > > = vec![vec![]];

    for track in tracks {
        let container = cosmic::widget::container::Container::new(
            cosmic::widget::row::with_children(vec![
                cosmic::widget::text::heading(format!(
                    "{}. {}",
                    track.track_number, track.name
                ))
                    .into(),
                cosmic::widget::horizontal_space().into(),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "media-playback-start-symbolic",
                ))
                    .on_press(Message::AddTrackToQueue(track.file_path.clone()))
                    .into(),
            ])
                .align_y(Alignment::Center),
        );
        log::info!("{}: {}", "Disc number".red(), track.disc_number);
        match discs.get_mut((track.disc_number - 1 )as usize) {
            None => {
                discs.push(vec![container]);
            }
            Some(val) => {
                val.push(container);
            }
        }
    }

    let mut disc_lists = vec![];
    for disc in discs {
        let mut list = Some(cosmic::widget::ListColumn::new());
        for element in disc {
            if let Some(new_list) = list.take() {
                let new_list = new_list.add(element);
               list = Some(new_list);
            }
        }
        disc_lists.push(list.unwrap().into_element());
    }

    cosmic::widget::column::with_children(disc_lists)
        .into()
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
    let conn = rusqlite::Connection::open(

        dirs::data_local_dir().unwrap().join(crate::app::AppModel::APP_ID).join("cosmic_music.db")

    ).unwrap();
    let row_num = conn
        .query_row(
            "SELECT * FROM album WHERE name = ?",
            [title.as_str()],
            |row| {
                Ok((
                    row.get::<usize, u32>(0),
                    row.get::<usize, u32>(3),
                    row.get::<usize, u32>(4),
                    row.get::<&str, Vec<u8>>("album_cover"),
                ))
            },
        )
        .unwrap();

    let album = Album {
        name: title,
        artist,
        disc_number: row_num.1.unwrap_or(0),
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

pub fn get_top_album_info(
    tx: &mut Sender<Message>,
    album_iter: Vec<(String, String, u32, u32, Option<Vec<u8>>)>,
) {

    // Prepare and execute query in a separate scope
    // Process results and send them
    let mut albums: Vec<Album> = vec![];

    for album_result in album_iter {


        albums.push(Album {
            name: album_result.0,
            artist: album_result.1,
            disc_number: album_result.2,
            track_number: album_result.3,
            cover_art: match album_result.4 {
                None => None,
                Some(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
            },
        })
    }

    tx.try_send(Message::AlbumProcessed(albums)).expect("Failed to send album process");
}
