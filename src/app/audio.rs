mod player;
mod queued_audio;
mod resampling;
pub(crate) mod tracktypes;

use crate::app::audio::queued_audio::QueuedAudio;
use crate::app::audio::tracktypes::AppTrack;
use crate::app::page::CoverArt;
use colored::Colorize;
use cosmic::widget::image::Handle;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use player::PlayerState;
use ringbuf::traits::Split;
use std::sync::Arc;
use symphonia::core::audio::conv::IntoSample;

#[derive(Debug, Clone)]
pub enum LoopState {
    LoopingTrack,
    LoopingQueue,
    NotLooping,
    RandomShuffle,
}

#[derive(Clone)]
pub struct CachedAudioMixer {
    pub app_track: AppTrack,
    pub sample: Vec<f32>,
}

pub struct AudioPlayer {
    pub queued_audio: QueuedAudio,
    pub player_state: PlayerState,
    pub stream_handle: Stream,
    pub stream_config: StreamConfig,
}

impl AudioPlayer {
    pub fn new(volume: f32) -> AudioPlayer {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("No output device available!");

        let config = match device.default_output_config() {
            Ok(a) => a.config(),
            Err(err) => {
                panic!("{}", err)
            }
        };

        let stream = device
            .build_output_stream(
                config,
                |input: &mut [f32], output| {},
                |err| panic!("{}", err),
                None,
            )
            .expect("Stream failed to build");

        AudioPlayer {
            player_state: PlayerState::new(),
            queued_audio: QueuedAudio::new(config.sample_rate as usize),
            stream_config: config,
            stream_handle: stream,
        }
    }

    fn read_write_stream(self) {
        let (write, consumer) = self.queued_audio.audio_ring.split();
    }

    pub fn play_now(&mut self, track_id: u32) -> Result<u32, String> {
        self.reset();
        let a = AppTrack::get_by_id(track_id)?;
        self.queued_audio.long_queue.push(a.to_queued_track());

        let next = self.queued_audio.get_next_cache_pointer();
        self.queued_audio.insert_cache_item(a, next);

        let a = self
            .queued_audio
            .decode_and_cache(self.stream_config.sample_rate);
        match a {
            Ok(a) => {
                log::info!("{}", a.bright_purple())
            }
            Err(a) => {
                log::info!("{}", "Something went wrong".red())
            }
        }

        Ok(1)
    }
    pub fn add_to_queue(&mut self, track_id: u32) {}
    pub fn reset(&mut self) {
        self.queued_audio.clear()
    }

    /// returns decorative elmeents, title, artist, album, cover
    pub fn display_current(&self) -> (Arc<String>, String, String, CoverArt) {
        let a = match self
            .queued_audio
            .cached
            .get(self.queued_audio.cache_pointer as usize)
            .unwrap()
            .as_ref()
        {
            None => {
                return (
                    Arc::from("".to_string()),
                    "".to_string(),
                    "".to_string(),
                    CoverArt::None,
                )
            }
            Some(a) => a,
        };

        return (
            a.app_track.title.clone(),
            a.app_track.artist.to_string(),
            a.app_track.album_title.to_string(),
            a.app_track.cover_art.clone(),
        );
    }
}
