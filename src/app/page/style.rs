// use crate::app::page::albums::AlbumPage;
// use crate::app::Message;
// use cosmic::iced::widget::scrollable::Viewport;
// use cosmic::iced::{Size, Subscription};
// use cosmic::Element;
//
// pub struct GridStyle {
//     page_items: ItemType,
//     thumbnail_jobs: u32,
//     viewport: Option<Viewport>,
//     size: Option<Size>,
// }
//
// pub trait GridPageStyle {
//     fn new_item_grid(&self) -> GridStyle;
// }
//
// impl GridPageStyle for AlbumPage {
//     fn new_item_grid(&self) -> GridStyle {
//         return GridStyle {
//             page_items: ItemType::Album(self.albums.clone()),
//             thumbnail_jobs: 0,
//             viewport: None,
//             size: None,
//         };
//     }
// }
//
// impl GridStyle {
//     pub fn view<'a>(self) -> Element<'a, Message> {
//         cosmic::widget::text::text("Hello!").into()
//     }
//
//     fn subscription(&self) -> Subscription<Message> {
//         let subscriptions = Vec::with_capacity(3 + self.thumbnail_jobs as usize);
//
//         // let visible_rect = {
//         //     let point: cosmic::iced::core::Point = match self.viewport {
//         //         None => {
//         //             cosmic::iced::core::Point { x: 0.0, y: 0.0 }
//         //         }
//         //         Some(viewport) => {
//         //             cosmic::iced::core::Point { x: 0.0, y: viewport.absolute_offset().y }
//         //         }
//         //     };
//         //
//         //     self.viewport.unwrap().
//         //     cosmi
//         //
//         // }
//         if let albums = ItemType::Album {}
//
//         return Subscription::batch(subscriptions);
//     }
// }
