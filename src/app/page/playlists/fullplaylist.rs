use crate::app::page::playlists::{Playlist, PlaylistTrack};
use crate::app::subpage::Subpage;
use crate::app::{AppModel, Message};

use crate::app::page::albums::fullalbum::display_track;
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
        "".to_string()
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        let len = self.tracks.len().clone();
        let mut column = cosmic::widget::Column::with_capacity(len);

        for track in &self.tracks {
            let elem = display_track(&track.title, track.id);
            column = column.push(elem)
        }

        return column.into();
    }

    fn data_ids(&self) -> Vec<u32> {
        log::info!("{:?}", self.tracks);
        self.tracks.iter().map(|a| a.id).collect::<Vec<u32>>()
    }
}
