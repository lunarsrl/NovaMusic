use crate::app::page::albums::FullAlbum;
use crate::app::subpage::Subpage;
use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::core::text::EllipsizeHeightLimit;
use cosmic::iced::widget::text::Ellipsize;
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::Element;
use std::collections::BTreeMap;
impl Subpage for FullAlbum {
    fn header_title(&self) -> String {
        return self.album.name.to_string();
    }
    fn header_image(&self) -> Element<Message> {
        match &self.album.cover_art {
            None => {
                return cosmic::widget::icon::from_name("applications-audio-symbolic")
                    .size(200)
                    .into();
            }
            Some(art) => {
                return cosmic::widget::image(art)
                    .height(Length::Fixed(170.0))
                    .width(Length::Fixed(170.0))
                    .content_fit(ContentFit::Cover)
                    .into();
            }
        }
    }

    fn header_subtitle(&self) -> String {
        return self.album.artist.to_string();
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        let mut discs: BTreeMap<u32, Vec<Element<Message>>> = BTreeMap::new();
        // let mut discs: Vec<Vec<Element<Message>>> =
        //     Vec::with_capacity(self.album.disc_number as usize);

        for track in &self.tracks {
            if let std::collections::btree_map::Entry::Vacant(e) = discs.entry(track.disc_number) {
                let new_disc: Vec<Element<Message>> = Vec::new();
                e.insert(new_disc);
                discs
                    .get_mut(&track.disc_number)
                    .unwrap()
                    .push(display_track(&track.name, track.id))
            } else {
                discs
                    .get_mut(&track.disc_number)
                    .unwrap()
                    .push(display_track(&track.name, track.id))
            }
        }

        let mut column = cosmic::widget::Column::with_capacity(self.album.track_number as usize);

        for (disc, tracks) in discs.into_iter() {
            column = column.push(display_disc(disc));
            for track in tracks {
                column = column.push(track)
            }
        }

        return column.into();
    }

    fn data_ids(&self) -> Vec<u32> {
        self.tracks.iter().map(|a| a.id).collect::<Vec<u32>>()
    }
}

fn display_disc<'a>(index: u32) -> Element<'a, Message> {
    cosmic::widget::column::with_children(vec![
        cosmic::widget::divider::horizontal::default().into(),
        cosmic::widget::row::with_children(vec![
            cosmic::widget::text::heading(fl!("AlbumDiscNumber", number = index.to_string()))
                .into(),
            cosmic::widget::space::horizontal().into(),
            cosmic::widget::button::text(fl!("AddToQueue"))
                .class(cosmic::widget::button::ButtonClass::Link)
                .into(),
            cosmic::widget::button::text(fl!("ReplaceQueue"))
                .class(cosmic::widget::button::ButtonClass::Link)
                .into(),
        ])
        .align_y(Vertical::Center)
        .into(),
        cosmic::widget::divider::horizontal::default().into(),
    ])
    .padding(cosmic::iced::padding::top(cosmic::theme::spacing().space_s))
    .into()
}

pub fn display_track(title: &str, id: u32) -> Element<Message> {
    let space_main = 300.0;
    let space_mod = 150.0;
    cosmic::iced::widget::hover(
        // Normal Display
        cosmic::widget::container(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::space::vertical().into(),
                cosmic::widget::text::heading(title)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                    .into(),
                cosmic::widget::space::vertical().into(),
                cosmic::widget::divider::horizontal::light().into(),
            ])
            .height(cosmic::theme::spacing().space_s + 24),
        )
        .align_x(Alignment::Center),
        // Display on hover, where the controls should be
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        cosmic::widget::container(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::space::vertical().into(),
                cosmic::widget::row::with_children([
                    cosmic::widget::text::heading(title)
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                        .into(),
                    cosmic::widget::space::horizontal().into(),
                    cosmic::widget::row::with_children(vec![
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("playlist-symbolic"),
                            ))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Add to playlist")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("list-add-symbolic"),
                            ))
                            .on_press(Message::AddTrackById(id))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Add to queue")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("media-playback-start-symbolic"),
                            ))
                            .on_press(Message::PlayTrackById(id))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Play now")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                    ])
                    .align_y(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_xxxs)
                    .into(), //right
                ])
                .align_y(Alignment::Center)
                .into(),
                cosmic::widget::space::vertical().into(),
                cosmic::widget::divider::horizontal::light().into(),
            ])
            .height(cosmic::theme::spacing().space_s + 24),
        )
        .class(cosmic::theme::Container::Secondary),
    )
}
