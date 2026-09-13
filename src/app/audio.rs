pub mod player;
pub mod queue;
mod resampling;
pub mod ringbuffer;
pub(crate) mod tracktypes;

use crate::app::audio::ringbuffer::AudioBuffer;
use crate::app::audio::tracktypes::AppTrack;
use colored::Colorize;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, Stream, StreamConfig};
use futures_util::future::err;
use rb::{RbConsumer, RbInspector, RB};
use rusqlite::fallible_iterator::FallibleIterator;
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
    pub read_head: u32,
    pub app_track: AppTrack,
    pub sample: Vec<f32>,
}

pub struct AudioPlayer {
    pub stream: Option<Stream>,
    pub device: Device,
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

        AudioPlayer {
            stream_config: config,
            device,
            stream: None,
        }
    }

    pub fn open_stream(&mut self, ring: Arc<AudioBuffer>) {
        if let Ok(stream) = self.device.build_output_stream(
            self.stream_config,
            move |out, _| {
                let ring = ring.ring.consumer();
                ring.read_blocking(out);
            },
            move |error| {
                log::error!(
                    "CPAL ERROR [{}]: {}",
                    error.kind(),
                    error.message().unwrap()
                )
            },
            None,
        ) {
            self.stream.replace(stream);
        } else {
            panic!("Failed to open output stream")
        }
    }

    pub fn play_now(&mut self, track_id: u32) -> Result<u32, String> {
        Ok(1)
    }
    pub fn add_to_queue(&mut self, track_id: u32) {}
    pub fn reset(&mut self) {}
}

pub fn decode_audio(id: u32, ring: Arc<AudioBuffer>) {}
