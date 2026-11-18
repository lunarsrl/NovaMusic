// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::subpage::SubpageBuilder;

pub(crate) mod fullplaylist;
use crate::app::page::playlists::fullplaylist::FullPlaylist;

use crate::app::page::tracks::SearchResult;
use crate::app::page::{BodyStyle, Page, PageBuilder};
use crate::app::{AppModel, Message};
use crate::fl;
use colored::Colorize;
use cosmic::iced::advanced::text::{Ellipsize, EllipsizeHeightLimit};
use cosmic::iced::widget::scrollable::Viewport;
use cosmic::iced::{Alignment, ContentFit, Length, Size};
use cosmic::widget::JustifyContent;
use cosmic::{Application, Element};
use std::cell::Cell;
use std::fs::File;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PlaylistPage {
    //data
    pub playlists: Arc<Vec<Playlist>>,
    pub playlist_page_state: PlaylistPageState,
    pub search_term: String,

    // view
    pub viewport: Option<Viewport>,
    pub scrollbar_id: cosmic::iced::widget::Id,
    size: Cell<Option<Size>>,
}

#[derive(Clone, Debug, Default)]
pub struct Playlist {
    pub title: String,
    pub path: String,
    pub thumbnail: Option<cosmic::widget::image::Handle>,
}

#[derive(Debug, Clone)]
pub struct PlaylistTrack {
    pub(crate) title: String,
    pub(crate) id: u32,
}

#[derive(Debug, Clone)]
pub enum PlaylistPageState {
    Loading,
    PlaylistPage(FullPlaylist),
    Search(Vec<SearchResult>),
}

const TextArea: f32 = 40.0;
impl Page for PlaylistPage {
    fn title(&self) -> String {
        return fl!("playlists");
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        let icon = model.config.grid_item_size;
        match &self.playlist_page_state {
            PlaylistPageState::Loading => cosmic::widget::responsive(move |size| {
                cosmic::widget::id_container::id_container(
                    self.view(size, icon),
                    cosmic::widget::Id::unique(),
                )
                .into()
            })
            .into(),
            _ => todo!("ijasdf"),
        }
    }

    fn body_style(&self) -> BodyStyle {
        BodyStyle::Grid
    }
}

impl PlaylistPage {
    pub fn new() -> PlaylistPage {
        PlaylistPage {
            playlists: Arc::new(vec![]),
            playlist_page_state: PlaylistPageState::Loading,
            scrollbar_id: cosmic::iced::widget::Id::unique(),
            viewport: None,
            search_term: "".to_string(),
            size: Cell::new(None),
        }
    }
    pub fn load_page(&self, model: &AppModel) -> Element<Message> {
        match &self.playlist_page_state {
            PlaylistPageState::Loading => self.page(model),
            PlaylistPageState::PlaylistPage(p) => p.page(model),
            _ => self.page(model),
        }
    }

    pub fn view<'a>(&self, size: Size, icon_size: u32) -> Element<'a, Message> {
        self.size.set(Some(size));

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
        //
        // log::info!(
        //     "OFFSET: {}",
        //     match self.viewport {
        //         None => {
        //             "NONE".red().to_string()
        //         }
        //         Some(a) => {
        //             a.absolute_offset().y.to_string().red().to_string()
        //         }
        //     }
        // );

        let visible_rect = cosmic::iced::Rectangle::new(
            cosmic::iced::Point::new(
                f32::from(cosmic::theme::spacing().space_s),
                match self.viewport {
                    None => 0.0,
                    Some(val) => val.absolute_offset().y,
                },
            ),
            cosmic::iced::Size::new(3.0, size.height),
        );

        let mut album_rect = cosmic::iced::Rectangle::new(
            cosmic::iced::Point::new(f32::from(cosmic::theme::spacing().space_s), 0.0),
            cosmic::iced::Size::new(3.0, icon_size as f32 * 32.0 + TextArea),
        );

        let mut grid = cosmic::widget::grid::<Message>()
            .column_spacing(spacing)
            .column_alignment(Alignment::Center)
            .justify_content(JustifyContent::Center)
            .row_alignment(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Shrink);

        for (index, playlist) in self.playlists.clone().iter().enumerate() {
            let insert_element;

            if album_rect.intersects(&visible_rect) {
                insert_element = playlist.display_grid(icon_size);
            } else {
                insert_element = cosmic::widget::Column::new()
                    .push(cosmic::widget::text(format!("{}", index)))
                    .width(Length::Fill)
                    .height(Length::Fixed(icon_size as f32 * 32.0 + TextArea))
                    .into()
            }

            item_num += 1;

            if item_num as u32 % items_per_row == 0 {
                grid = grid.push(insert_element).insert_row();
                album_rect.y += icon_size as f32 * 32.0 + TextArea + 4f32;
            } else {
                grid = grid.push(insert_element);
            }
        }

        cosmic::widget::scrollable::vertical(grid)
            .id(self.scrollbar_id.clone())
            .on_scroll(|view| Message::ScrollView(view))
            .into()
    }

    pub fn load_page_data(&mut self) {
        let mut page_data: Vec<Playlist> = vec![];

        let dir = dirs::data_local_dir()
            .unwrap()
            .join(crate::app::AppModel::APP_ID)
            .join("Playlists");

        if dir.is_dir() {
            let dir_tree = dir.read_dir().unwrap();

            dir_tree.filter_map(|a| a.ok()).for_each(|file| {
                let mut playlist = Playlist::default();

                let mut real_file = File::open(file.path()).unwrap();
                let mut file_buf: String = "".to_string();
                real_file.read_to_string(&mut file_buf);

                let file_lines = file_buf.lines();
                for line in file_lines {
                    // Title
                    if line.starts_with("#PLAYLIST:") {
                        let title = line.trim_prefix("#PLAYLIST:");
                        playlist.title = title.to_string();
                        continue;
                    }
                    // Cover art
                    if line.starts_with("#EXTALBUMARTURL:") {
                        let path = line.trim_prefix("#EXTALBUMARTURL:");
                        let real_path = path.parse::<PathBuf>().unwrap();
                        let handle = cosmic::widget::image::Handle::from_path(real_path);
                        playlist.thumbnail.replace(handle);
                        continue;
                    }
                }
                playlist.path = file.path().to_str().unwrap().to_string();
                page_data.push(playlist)
            });

            self.playlists = Arc::new(page_data)
        } else {
        }
    }
}

impl Playlist {
    fn display_grid<'a>(&self, size: u32) -> Element<'a, Message> {
        let art: Element<Message> = match &self.thumbnail {
            None => {
                cosmic::widget::container(
                    cosmic::widget::icon::from_name("audio-x-generic")
                        .icon()
                        .width(Length::Fixed(size as f32 * 24.0))
                        .height(Length::Fixed(size as f32 * 24.0)),
                )
                .center(Length::Fixed(size as f32 * 24.0))
            }
            .align_x(cosmic::iced::Alignment::Center)
            .into(),
            Some(art) => cosmic::widget::image(art)
                .content_fit(ContentFit::Contain)
                .width(Length::Fixed(size as f32 * 32.0))
                .height(Length::Fixed(size as f32 * 32.0))
                .into(),
        };

        cosmic::widget::container(
            cosmic::widget::button::custom(
                cosmic::widget::column::with_children(vec![
                    art,
                    cosmic::widget::text::caption_heading(self.title.to_string())
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                        .into(),
                ])
                .align_x(cosmic::iced::Alignment::Center),
            )
            .on_press(Message::PlaylistSelected(self.clone()))
            .class(cosmic::theme::Button::IconVertical)
            .width(Length::Fixed(size as f32 * 32.0)),
        )
        .height(Length::Fixed(size as f32 * 32.0 + TextArea))
        .width(Length::Fixed(size as f32 * 32.0))
        .into()
    }
}
