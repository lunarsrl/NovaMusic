// SPDX-License-Identifier: GPL-2.0-or-later

pub mod albums;
pub mod artists;
pub mod genre;
pub mod playlists;
mod style;
pub mod tracks;

use crate::app::page::albums::Album;
use crate::app::page::artists::ArtistInfo;
use crate::app::page::genre::GenrePage;
use crate::app::page::playlists::PlaylistPage;
use crate::app::{AppModel, Message};
use crate::config::SortOrder;
use crate::fl;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::widget::shader::Viewport;
use cosmic::iced::{Alignment, Length, Size};
use cosmic::widget::Id;
use cosmic::{iced, Element};
use std::fmt::Display;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoverArt {
    None,
    SomeUnloaded,
    SomeLoaded(cosmic::widget::image::Handle),
}
enum ItemType {
    Album(Arc<RwLock<Vec<Album>>>),
    Artist(ArtistInfo),
}

enum BodyStyle {
    Grid,
    List,
}

pub struct PageType {
    page_title: String,
    data_stored: ItemType,
    body_style: BodyStyle,
    scrollbar_id: Id,
    viewport: Viewport,
    size: Size,
}

impl PageType {
    pub fn view(&self) -> Element<'_, Message> {
        match self.body_style {
            BodyStyle::Grid => {}
            BodyStyle::List => {
                todo!()
            }
        }
    }
}

pub trait Page {
    fn title(&self) -> String;
    fn body(&self, model: &AppModel) -> Element<Message>;
    fn body_style(&self) -> BodyStyle;
}

trait PageBuilder {
    fn page(&self, model: &AppModel) -> Element<Message>;
    fn header(&self) -> Element<Message>;
}

impl<T: Page> PageBuilder for T {
    fn page(&self, model: &AppModel) -> Element<Message> {
        let sticky_elements = match self.body_style() {
            BodyStyle::Grid => {
                cosmic::widget::container(cosmic::widget::column::with_children(vec![])).into()
            }
            BodyStyle::List => list_sort_header(
                "Title".to_string(),
                "Modifiable 1".to_string(),
                "Modifiable 2".to_string(),
                model.config.sort_order,
            ),
        };

        match self.body_style() {
            BodyStyle::Grid => cosmic::widget::container(
                cosmic::widget::column::with_children(vec![
                    self.header(),
                    cosmic::widget::container(sticky_elements)
                        .padding(iced::core::padding::Padding::from([
                            0,
                            cosmic::theme::spacing().space_xxs,
                        ]))
                        .into(),
                    cosmic::widget::scrollable::vertical(
                        cosmic::widget::container(self.body(model))
                            .padding(iced::core::padding::Padding::from([
                                0,
                                cosmic::theme::spacing().space_s,
                            ]))
                            .height(Length::Fill)
                            .width(Length::Fill),
                    )
                    .into(),
                ])
                .spacing(cosmic::theme::spacing().space_xs),
            )
            .into(),
            BodyStyle::List => cosmic::widget::column::with_children(vec![
                cosmic::widget::column::with_children(vec![
                    self.header(),
                    cosmic::widget::container(sticky_elements).into(),
                ])
                .padding(iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_s,
                ]))
                .into(),
                cosmic::widget::scrollable::vertical(
                    cosmic::widget::container(self.body(model))
                        .padding(iced::core::padding::Padding::from([
                            0,
                            cosmic::theme::spacing().space_s,
                        ]))
                        .height(Length::Fill)
                        .width(Length::Fill),
                )
                .height(Length::Shrink)
                .on_scroll(|a| Message::ScrollView(a))
                .into(),
            ])
            .into(),
        }
    }

    fn header(&self) -> Element<Message> {
        cosmic::widget::column::with_children(vec![
            cosmic::widget::row::with_children(vec![cosmic::widget::text::title3(self.title())
                .width(Length::FillPortion(2))
                .into()])
            .align_y(Alignment::Center)
            .spacing(cosmic::theme::spacing().space_xxs)
            .into(),
            cosmic::widget::divider::horizontal::default().into(),
        ])
        .spacing(cosmic::theme::spacing().space_xxs)
        .into()
    }
}

// --------------------------------------------------- Pages implemented for -----------------------

impl Page for GenrePage {
    fn title(&self) -> String {
        String::from(fl!("genres"))
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        todo!()
    }

    fn body_style(&self) -> BodyStyle {
        return BodyStyle::Grid;
    }
}

impl Page for PlaylistPage {
    fn title(&self) -> String {
        String::from(fl!("playlists"))
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        todo!()
    }

    fn body_style(&self) -> BodyStyle {
        return BodyStyle::Grid;
    }
}

// useful stuff for multiple pages
pub fn list_sort_header<'a>(
    field1: String,
    field2: String,
    field3: String,
    selection: crate::config::SortOrder,
) -> Element<'a, Message> {
    let sort_order_icon = match selection {
        SortOrder::Ascending => cosmic::widget::icon::from_name("pan-down-symbolic").into(),
        SortOrder::Descending => cosmic::widget::icon::from_name("pan-up-symbolic").into(),
    };

    return cosmic::widget::column::with_children(vec![
        cosmic::widget::row::with_children(vec![
            cosmic::widget::button::custom(
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::text::heading(field1).into(),
                    sort_order_icon,
                ])
                .align_y(Vertical::Center),
            )
            .on_press(match selection {
                SortOrder::Ascending => Message::Sort(SortOrder::Descending),
                SortOrder::Descending => Message::Sort(SortOrder::Ascending),
            })
            .width(Length::Fixed(300.0))
            .class(cosmic::theme::Button::MenuRoot)
            .into(),
            cosmic::widget::space().width(Length::Fill).into(),
            // cosmic::widget::button::custom(
            //     cosmic::widget::row::with_children(vec![cosmic::widget::text::heading(
            //         "Modifiable",
            //     )
            //     .into()])
            //     .align_y(Vertical::Center),
            // )
            // .width(Length::Fixed(150.0))
            // .class(cosmic::theme::Button::MenuRoot)
            // .into(),
            // cosmic::widget::space().width(Length::Fill).into(),
            // cosmic::widget::button::custom(
            //     cosmic::widget::row::with_children(vec![cosmic::widget::text::heading(
            //         "Modifiable",
            //     )
            //     .into()])
            //     .align_y(Vertical::Center),
            // )
            // .width(Length::Fixed(150.0))
            // .class(cosmic::theme::Button::MenuRoot)
            // .into(),
            cosmic::widget::space().width(Length::Fill).into(),
        ])
        .align_y(Alignment::Center)
        .into(),
        cosmic::widget::divider::horizontal::default().into(),
    ])
    .into();
}
