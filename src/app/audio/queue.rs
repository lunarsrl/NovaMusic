use crate::app::audio::tracktypes::{AppTrack, QueuedTrack};
use crate::app::page::CoverArt;

pub struct AudioQueue {
    pub queue_pos: u32,
    upcoming_cur: bool,
    pub(crate) long_queue: Vec<QueuedTrack>,
    upcoming: [Option<AppTrack>; 2],
}

impl AudioQueue {
    pub fn new() -> AudioQueue {
        AudioQueue {
            queue_pos: 0,
            upcoming_cur: false,
            long_queue: vec![],
            upcoming: [None, None],
        }
    }

    pub fn append(&mut self, new: QueuedTrack) {
        if !(self.long_queue.len() >= 2) {
            self.cycle_upcoming(new.to_app_track())
        }
        self.long_queue.push(new);
    }

    fn cycle_upcoming(&mut self, track: AppTrack) {
        self.upcoming
            .get_mut((!self.upcoming_cur) as usize)
            .expect("Array should always be initialied")
            .replace(track);
    }

    pub(crate) fn display_current(&self) -> Option<(String, String, String, CoverArt)> {
        if let Some(a) = self
            .upcoming
            .get(self.upcoming_cur as usize)
            .expect("Array should always be initialized")
        {
            return Some((
                a.title.to_string(),
                a.artist.to_string(),
                a.album_title.to_string(),
                a.cover_art.clone(),
            ));
        } else {
            return None;
        }
    }
    /// Should be run after q song is finished
    fn next(&mut self) {
        self.upcoming_cur = !self.upcoming_cur;
        if !self.long_queue.is_empty() {
            if let Some(track) = self.long_queue.get(self.queue_pos as usize) {
                self.cycle_upcoming(track.to_app_track())
            }
        }
    }
}
