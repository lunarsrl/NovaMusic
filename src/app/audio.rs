pub mod queue;
mod resampling;
pub mod ringbuffer;
pub mod states;
pub(crate) mod tracktypes;

use crate::app::audio::resampling::Resamplifier;
use crate::app::audio::ringbuffer::AudioBuffer;
use crate::app::audio::tracktypes::AppTrack;
use crate::app::{Message, QueueUpdateReason};
use colored::Colorize;
use cosmic::cosmic_theme::palette::chromatic_adaptation::AdaptIntoUnclamped;
use cosmic::iced::runtime::task::widget;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, Stream, StreamConfig};
use futures::channel::mpsc::{Receiver, SendError, Sender};
use futures_util::future::err;
use futures_util::{SinkExt, TryFutureExt};
use rand::seq::index::sample;
use rb::{RbConsumer, RbInspector, RbProducer, RB};
use rusqlite::fallible_iterator::FallibleIterator;
use std::fmt::format;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use symphonia::core::audio::conv::IntoSample;
use symphonia::core::audio::AmbisonicBFormat::T;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::TrackType;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use tokio::sync::mpsc::UnboundedSender;

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
    pub stream: Option<Arc<Stream>>,
    pub device: Device,
    pub stream_config: StreamConfig,
    samples_sampled: Arc<Mutex<usize>>,
    samples_remaining: Arc<Mutex<usize>>,
    sender: Arc<Mutex<Sender<Message>>>,
}

impl AudioPlayer {
    pub fn new(volume: f32) -> (AudioPlayer, Receiver<Message>) {
        let (tx, rx) = futures::channel::mpsc::channel(0);
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

        (
            AudioPlayer {
                stream: None,
                device,
                stream_config: config,
                samples_sampled: Arc::new(Mutex::new(0)),
                samples_remaining: Arc::new(Mutex::new(0)),
                sender: Arc::new(Mutex::new(tx)),
            },
            rx,
        )
    }

    pub fn link_to_buffer(&mut self, ring: Arc<AudioBuffer>) {
        let remaining = Arc::clone(&self.samples_remaining);
        let sampled = Arc::clone(&self.samples_sampled);
        let mut sender = Arc::clone(&self.sender);
        if let Ok(stream) = self.device.build_output_stream(
            self.stream_config,
            move |out, a| {
                let rring = ring.ring.consumer();
                let write = rring.read(out).unwrap_or(0);

                // log::info!(
                //     "{} Size of read: {}, Samples to read: {}",
                //     "[Audio Callback]".yellow(),
                //     write,
                //     ring.ring.count()
                // );

                out[write..]
                    .iter_mut()
                    .for_each(|s| *s = Sample::EQUILIBRIUM);

                if let Ok(mut remaining) = remaining.lock() {
                    if *remaining == 0 {
                        return;
                    }

                    if let Ok(mut sampled_samples) = sampled.lock() {
                        *sampled_samples += write;
                        log::info!(
                            "{} Transition Progress: {}/{}",
                            "[Audio Callback]".yellow(),
                            sampled_samples,
                            remaining
                        );
                        if *sampled_samples > *remaining {
                            if let Ok(mut a) = sender.lock() {
                                (*a).start_send(Message::SongFinished(
                                    QueueUpdateReason::SongFinished,
                                ));
                            }

                            log::info!("{} Transition Donw!", "[Audio Callback]".yellow(),);
                            *sampled_samples = 0;
                            *remaining = 0;
                        }
                    }
                }
            },
            move |error| {
                log::error!(
                    "CPAL ERROR [{}]: {}",
                    error.kind(),
                    error.message().expect("No error message somehow")
                )
            },
            None,
        ) {
            self.stream.replace(Arc::from(stream));
        } else {
            panic!("Failed to open output stream")
        }
    }
    pub fn play_now(&mut self, track_id: u32) -> Result<u32, String> {
        Ok(1)
    }
    pub fn add_to_queue(&mut self, track_id: u32) {}
    pub fn reset(&mut self) {}

    pub fn track_samples(&mut self, samples: usize) {
        *self.samples_remaining.lock().unwrap() = samples;
        log::info!(
            "{}: {}",
            "Now tracking samples".green(),
            self.samples_remaining.lock().expect("Hi").to_string()
        )
    }
}

pub async fn decode_audio(
    file: Arc<PathBuf>,
    ring: Arc<AudioBuffer>,
    sample_rate: u32,
    tx: &mut Sender<Message>,
) {
    tx.send(Message::ToastError("BYEEEE!".to_string()))
        .await
        .expect("sdfahi");
    let mut messages: Vec<Message> = vec![];
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

    if let Some(dur) = track.duration {
        if let Some(timebase) = track.time_base {
            let time = timebase.calc_duration(dur).unwrap().as_secs();
            log::info!("Time: {}", time);

            tx.send(Message::AudioDuration(time)).await.expect("HI");
        } else {
            log::info!("No time")
        }
    } else {
        log::info!("No time")
    }

    let codecs = track.codec_params.clone().expect("Failed to return codec ");
    let audio_param = codecs
        .audio()
        .expect("Fauled to return audio_codec")
        .clone();

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio_param, &Default::default())
        .expect("Failed to make decoder");

    let channels = audio_param.channels.unwrap();
    let track_rate = audio_param.sample_rate.expect("No defined sample rate");
    let resample = !(track_rate == sample_rate);
    let time_base = track.time_base;

    // First packet outside of loop to initialize resampler
    let mut resamplething: Option<Resamplifier> = None;
    if let Some(first_packet) = res.next_packet().expect("Packets") {
        let first_audio = decoder.decode(&first_packet);

        if resample {
            resamplething.replace(Resamplifier::new(
                track_rate as usize,
                sample_rate as usize,
                channels.clone(),
                first_audio.unwrap().capacity(),
            ));
        }
    }

    while let Some(packet) = res.next_packet().expect("a") {
        match decoder.decode(&packet) {
            Ok(audio) => {
                if let Some(timeconv) = time_base {
                    let time = timeconv.calc_duration(packet.dur);
                    if let Some(time) = time {
                        // log::info!("{} {}ms worth of audio", "[Writer]".red(), time.as_millis())
                    }
                }

                let mut out: Vec<f32> = Vec::with_capacity(audio.samples_interleaved());

                if let Some(resampler) = resamplething.as_mut() {
                    // log::info!(
                    //     "{}: {}",
                    //     "[Writer]".red(),
                    //     "Must be resampled first!".bright_yellow()
                    // );
                    resampler.resample(audio, &mut out);

                    // log::info!("{}: {}", "[Writer]".red(), "Ready to copy!".green());
                } else {
                    audio.copy_to_vec_interleaved(&mut out);
                }

                while ring.ring.capacity() - ring.ring.count() < out.len() {
                    // log::info!(
                    // "{} Not enough space! {} vs {}",
                    // "[Writer]".red(),
                    // ring.ring.capacity() - ring.ring.count(),
                    // out.len()
                    // );
                    sleep(Duration::from_millis(1));
                }

                let write = ring.ring.producer().write(out.as_slice());
                // log::info!(
                //     "{} Size of Write: {}",
                //     "[Writer]".red(),
                //     write.unwrap_or(0).to_string()
                // )
            }
            Err(err) => {
                log::warn!("Failed to read packet: {}", err);
                continue;
            }
        }
    }
}
