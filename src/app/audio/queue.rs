use crate::app::audio::decode_audio;
use crate::app::audio::tracktypes::{AppTrack, QueuedTrack};
use crate::app::page::CoverArt;
use crate::app::{AppModel, Message, QueueUpdateReason};
use colored::Colorize;
use cosmic::cosmic_theme::palette::chromatic_adaptation::AdaptIntoUnclamped;
use cosmic::Application;
use futures::channel::mpsc::Sender;
use futures_util::SinkExt;
use std::time::Duration;

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
        log::info!("{}", "Track Appended!".green());
        if !(self.long_queue.len() >= 2) {
            self.cycle_upcoming(new.to_app_track())
        }
        self.long_queue.push(new);
        log::info!(
            "AudioQueue State \nLong: {:#?}\nCached: {:#?}, {:#?}",
            self.long_queue,
            self.upcoming.get(0).unwrap(),
            self.upcoming.get(1).unwrap()
        )
    }

    fn cycle_upcoming(&mut self, track: AppTrack) {
        match self
            .upcoming
            .get_mut((self.upcoming_cur) as usize)
            .expect("Array should always be initialied")
        {
            None => {
                self.upcoming
                    .get_mut(self.upcoming_cur as usize)
                    .expect("should always be init")
                    .replace(track);
            }
            Some(_) => {
                self.upcoming
                    .get_mut(!self.upcoming_cur as usize)
                    .expect("should always be good")
                    .replace(track);
            }
        }
    }

    pub(crate) fn display_current(&self) -> Option<(String, String, String, CoverArt)> {
        self.upcoming
            .get(self.upcoming_cur as usize)
            .expect("Array should always be initialized")
            .as_ref()
            .map(|a| {
                (
                    a.title.to_string(),
                    a.artist.to_string(),
                    a.album_title.to_string(),
                    a.cover_art.clone(),
                )
            })
    }
    /// Should be run after q song is finished
    pub fn next(&mut self) -> bool {
        self.queue_pos += 1;
        self.upcoming_cur = !self.upcoming_cur;

        if !self.long_queue.is_empty() || self.long_queue.get(self.queue_pos as usize).is_none() {
            if let Some(track) = self.long_queue.get(self.queue_pos as usize) {
                self.cycle_upcoming(track.to_app_track());

                true
            } else {
                false
            }
        } else {
            self.queue_pos = 0;
            false
        }
    }

    pub fn to_decode(&self, model: &AppModel) -> cosmic::Task<cosmic::Action<Message>> {
        let path;
        if let Some(a) = self
            .upcoming
            .get(self.upcoming_cur as usize)
            .expect("Array should always be initialized")
        {
            path = a.path_buf.clone();
        } else {
            return cosmic::task::future(async move {
                Message::ToastError(
                    "Something went wrong queing up the next audiasfdoiajdsf".to_string(),
                )
            })
            .map(cosmic::Action::App);
        }

        let buf = model.audio_buffer.clone();
        let samp = model.audio_player.stream_config.sample_rate.clone();

        cosmic::task::stream(cosmic::iced::stream::channel(
            1,
            move |mut tx: Sender<Message>| async move {
                decode_audio(path.clone(), buf.clone(), samp, &mut tx).await;
            },
        ))
        .map(cosmic::Action::App)
    }
}
