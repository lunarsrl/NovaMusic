// SPDX-License-Identifier: GPL-2.0-or-later

pub mod albums;
pub mod artists;
pub mod genre;
pub mod playlists;
pub mod tracks;

use crate::app::page::albums::AlbumPage;
use crate::app::page::artists::ArtistsPage;
use crate::app::page::genre::GenrePage;
use crate::app::page::playlists::PlaylistPage;
use crate::app::Message;
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::{iced, Element};
use std::fmt::Display;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoverArt {
    None,
    SomeUnloaded,
    SomeLoaded(cosmic::widget::image::Handle),
}

enum PageBodies {
    Grid,
    List,
    Card,
}

pub trait Page {
    fn title(&self) -> String;
    fn body(&self) -> Element<Message>;
}

trait PageBuilder {
    fn page(&self) -> Element<Message>;
    fn header(&self) -> Element<Message>;
}

impl<T: Page> PageBuilder for T {
    fn page(&self) -> Element<Message> where {
        cosmic::widget::container(cosmic::widget::column::with_children(vec![
            self.header(),
            self.body(),

        ]))
        .height(Length::Fill)
        .width(Length::Fill)
        .padding(iced::core::padding::Padding::from([
            0,
            cosmic::theme::spacing().space_m,
        ]))
        .into()
    }

    fn header(&self) -> Element<Message> {
        cosmic::widget::column::with_children(vec![
            cosmic::widget::row::with_children(vec![
                cosmic::widget::text::title3(self.title())
                    .width(Length::FillPortion(2))
                    .into(),
                cosmic::widget::horizontal_space()
                    .width(Length::Shrink)
                    .into(),
            ])
            .align_y(Alignment::Center)
            .spacing(cosmic::theme::spacing().space_s)
            .into(),
            cosmic::widget::divider::horizontal::light().into(),
        ])
        .spacing(cosmic::theme::spacing().space_s)
        .into()
    }
}

// --------------------------------------------------- Pages implemented for -----------------------

impl Page for ArtistsPage {
    fn title(&self) -> String {
        String::from(fl!("artists"))
    }

    fn body(&self) -> Element<Message> {
        todo!()
    }
}

impl Page for GenrePage {
    fn title(&self) -> String {
        String::from(fl!("genres"))
    }

    fn body(&self) -> Element<Message> {
        todo!()
    }
}

impl Page for PlaylistPage {
    fn title(&self) -> String {
        String::from(fl!("playlists"))
    }

    fn body(&self) -> Element<Message> {
        todo!()
    }
}
impl Page for AlbumPage {
    fn title(&self) -> String {
        String::from(fl!("AlbumLibrary"))
    }

    fn body(&self) -> Element<Message> {
        todo!()
    }
}
