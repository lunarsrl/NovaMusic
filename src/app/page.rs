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
use cosmic::iced::alignment::Vertical;
use cosmic::widget::JustifyContent;
use crate::config::SortOrder;

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
            cosmic::widget::divider::horizontal::heavy().into(),
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


// useful stuff for multiple pages
pub fn list_sort_header<'a>(field1: String, field2: String, field3: String, selection: crate::config::SortOrder) -> Element<'a, Message>{
    let sort_order_icon = match selection {
        SortOrder::Ascending => {
            cosmic::widget::icon::from_name("pan-down-symbolic").into()
        }
        SortOrder::Descending => {
            cosmic::widget::icon::from_name("pan-up-symbolic").into()
        }
    };

    return cosmic::widget::flex_row(vec![
        cosmic::widget::button::custom(
            cosmic::widget::row::with_children(vec![
                cosmic::widget::text::heading(field1).into(),
                sort_order_icon
            ]).align_y(Vertical::Center)
        ).class(cosmic::theme::Button::MenuRoot)
            .into(),
        cosmic::widget::button::custom(
            cosmic::widget::row::with_children(vec![
                cosmic::widget::text::heading("Modifiable Field 1").into(),
            ]).align_y(Vertical::Center)
        ).class(cosmic::theme::Button::MenuRoot)
            .into(),
        cosmic::widget::button::custom(
            cosmic::widget::row::with_children(vec![
                cosmic::widget::text::heading("Modifiable Field 2 ").into(),
            ]).align_y(Vertical::Center)
        ).class(cosmic::theme::Button::MenuRoot)
            .into(),
    ]).justify_content(JustifyContent::SpaceBetween)
        .align_items(Alignment::Center)
        .into();
}