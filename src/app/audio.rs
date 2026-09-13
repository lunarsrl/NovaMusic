pub mod queue;
mod resampling;
pub mod ringbuffer;
pub mod states;
pub(crate) mod tracktypes;

use crate::app::audio::ringbuffer::AudioBuffer;
use crate::app::audio::tracktypes::AppTrack;
use colored::Colorize;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, Stream, StreamConfig};
use futures_util::future::err;
use rand::seq::index::sample;
use rb::{RbConsumer, RbInspector, RbProducer, RB};
use rusqlite::fallible_iterator::FallibleIterator;
use std::path::PathBuf;
use std::sync::Arc;
use symphonia::core::audio::conv::IntoSample;
use symphonia::core::audio::AmbisonicBFormat::T;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::TrackType;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use crate::app::audio::resampling::Resamplifier;

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
            move |out, a| {
                let rring = ring.ring.consumer();
                let write = rring.read(out).unwrap_or(0);
                log::info!(
                    "[{}] Size of read: {}, Samples to read: {}",
                    ring.ring.is_full().to_string(),
                    write.to_string().yellow(),
                    ring.ring.count()
                );
                out[write..]
                    .iter_mut()
                    .for_each(|s| *s = Sample::EQUILIBRIUM);
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

pub fn decode_audio(file: Arc<PathBuf>, ring: Arc<AudioBuffer>, sample_rate: u32) {
    let file = std::fs::File::open(file.to_path_buf()).expect("Failed to open file");
    let probe = symphonia::default::get_probe();

    let mss = MediaSourceStream::new(Box::from(file), Default::default());

    let mut res = probe
        .probe(
            &Default::default(),
            mss,
            Default::default(),
            Default::default(),
        )
        .expect("Failed to probe file");

    let track = res
        .default_track(TrackType::Audio)
        .expect("Track has no default audio track");

    let codecs = track.codec_params.clone().expect("Failed to return codec ");
    let audio = codecs.audio().expect("Fauled to return audio_codec");

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio, &Default::default())
        .expect("Failed to make decoder");


    let track_rate = audio.sample_rate.expect("No defined sample rate")
    let resample = !(track_rate == sample_rate);



    Resamplifier::new(track_rate as usize, sample_rate as usize, audio.channels.unwrap(), 0);

    while let Some(packet) = res.next_packet().expect("a") {
        match decoder.decode(&packet) {
            Ok(audio) => {
                let mut out: Vec<f32> = Vec::with_capacity(audio.samples_interleaved());


                if resample {
                } else {
                    audio.copy_to_vec_interleaved(&mut out);
                    let write = ring.ring.producer().write_blocking(out.as_slice());
                    log::info!("Size of write: {}", write.unwrap_or(0).to_string().red());
                }
            }
            Err(err) => {
                log::warn!("Failed to read packet: {}", err);
                continue;
            }
        }
    }
}
