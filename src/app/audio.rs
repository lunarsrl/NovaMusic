mod player;
mod queued_audio;
mod resampling;
pub(crate) mod tracktypes;

use crate::app::audio::queued_audio::QueuedAudio;
use crate::app::audio::tracktypes::AppTrack;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use player::PlayerState;
use symphonia::core::audio::conv::IntoSample;

#[derive(Debug, Clone)]
pub enum LoopState {
    LoopingTrack,
    LoopingQueue,
    NotLooping,
    RandomShuffle,
}

pub struct CachedAudio {
    pub app_track: AppTrack,
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

        log::info!("streammmmmm: {}", config.sample_rate);

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
    pub fn stream_data(self) {}
    pub fn play_now(&mut self, track_id: u32) -> Result<u32, String> {
        self.reset();
        let a = AppTrack::get_by_id(track_id)?;
        self.queued_audio.long_queue.push(a.to_queued_track());
        self.queued_audio.cached.push(CachedAudio { app_track: a });
        self.queued_audio.decode(self.stream_config.sample_rate);

        Ok(1)
    }
    pub fn add_to_queue(&mut self, track_id: u32) {}
    pub fn reset(&mut self) {
        self.queued_audio.queue_pos = 0;
        self.queued_audio.cached.clear();
        self.queued_audio.long_queue.clear();
        self.player_state.song_progress = 0.0;
    }
}
