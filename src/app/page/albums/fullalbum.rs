use crate::app::page::albums::FullAlbum;
use crate::app::subpage::Subpage;
use crate::app::{AppModel, Message};
use cosmic::iced::{ContentFit, Length};
use cosmic::Element;

impl Subpage for FullAlbum {
    fn header_title(&self) -> String {
        return self.album.name.to_string();
    }

    fn header_image(&self) -> Element<Message> {
        match &self.album.cover_art {
            None => {
                return cosmic::widget::icon::from_name("applications-audio-symbolic")
                    .size(200)
                    .into()
            }
            Some(art) => {
                return cosmic::widget::image(art)
                    .height(Length::Fixed(170.0))
                    .width(Length::Fixed(170.0))
                    .content_fit(ContentFit::Cover)
                    .into()
            }
        }
    }

    fn header_subtitle(&self) -> String {
        return self.album.artist.to_string();
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        cosmic::widget::text("are we vibing with the subpage layouts").into()
    }
}
