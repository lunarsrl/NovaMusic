// SPDX-License-Identifier: GPL-2.0-or-later

pub mod albums;
pub mod artists;
pub mod genre;
pub mod playlists;
pub mod tracks;

// SPDX-License-Identifier: GPL-2.0-or-later
use crate::app::page::albums::AlbumPage;
use crate::app::page::artists::ArtistsPage;
use crate::app::page::genre::GenrePage;
use crate::app::page::playlists::PlaylistPage;
use crate::app::page::tracks::TrackPage;
use crate::app::Message;
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::{iced, Element};
use std::fmt::Display;

pub trait Page {
    fn title(&self) -> String;
    fn search(&self) -> &str;
}

pub trait PageBuilder {
    fn page(&self) -> Element<Message>;
    fn header(&self, search: String) -> Element<Message>;
    fn body(&self) -> Element<Message>;
}

impl<T: Page> PageBuilder for T {
    fn page(&self) -> Element<Message> {
        todo!()
    }

    fn header(&self, search: String) -> Element<Message> {
        cosmic::widget::container(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text::title3(self.title())
                        .width(Length::FillPortion(2))
                        .into(),
                    cosmic::widget::horizontal_space()
                        .width(Length::Shrink)
                        .into(),
                    cosmic::widget::search_input(fl!("gensearch"), self.search().to_string())
                        .on_input(|input| Message::UpdateSearch(input))
                        .width(Length::FillPortion(1))
                        .into(),
                ])
                .align_y(Alignment::Center)
                .spacing(cosmic::theme::spacing().space_s)
                .into(),
                cosmic::widget::divider::horizontal::light().into(),
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

    fn body(&self) -> Element<Message> {
        todo!()
    }
}

// --------------------------------------------------- Pages implemented for -----------------------
impl Page for TrackPage {
    fn title(&self) -> String {
        String::from(fl!("TrackLibrary"))
    }
    fn search(&self) -> &str {
        self.SearchTerm.as_str()
    }
}

impl Page for ArtistsPage {
    fn title(&self) -> String {
        String::from(fl!("artists"))
    }
    fn search(&self) -> &str {
        self.search_term.as_str()
    }
}

impl Page for GenrePage {
    fn title(&self) -> String {
        String::from(fl!("genres"))
    }
    fn search(&self) -> &str {
        self.search_term.as_str()
    }
}

impl Page for PlaylistPage {
    fn title(&self) -> String {
        String::from(fl!("playlists"))
    }
    fn search(&self) -> &str {
        self.search_term.as_str()
    }
}
impl Page for AlbumPage {
    fn title(&self) -> String {
        String::from(fl!("AlbumLibrary"))
    }
    fn search(&self) -> &str {
        self.search_term.as_str()
    }
}
