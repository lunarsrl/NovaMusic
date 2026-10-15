use crate::app::page::playlists::{Playlist, PlaylistTrack};
use crate::app::subpage::Subpage;
use crate::app::{AppModel, Message};
use cosmic::Element;

#[derive(Debug, Clone)]
pub struct FullPlaylist {
    pub playlist: Playlist,
    pub tracks: Vec<PlaylistTrack>,
}

impl Subpage for FullPlaylist {
    fn header_title(&self) -> String {
        return self.playlist.title.to_string();
    }

    fn header_image(&self) -> Element<Message> {
        match self.playlist.thumbnail.as_ref() {
            None => cosmic::widget::icon::from_name("sodif").into(),
            Some(image) => cosmic::widget::image(image).into(),
        }
    }

    fn header_subtitle(&self) -> String {
        self.playlist.path.to_string()
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        cosmic::widget::text("Whats up chat").into()
    }

    fn data_ids(&self) -> Vec<u32> {
        todo!()
    }
}
