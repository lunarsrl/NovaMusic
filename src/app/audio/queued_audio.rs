use crate::app::audio::CachedAudio;

use crate::app::audio::resampling::Resamplifier;
use crate::app::audio::tracktypes::QueuedTrack;
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use std::fs::File;
use symphonia::core::audio::GenericAudioBuffer;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::TrackType;
use symphonia::core::io::MediaSourceStream;
use symphonia::default::get_codecs;

pub struct QueuedAudio {
    audio_ring: HeapRb<GenericAudioBuffer>,
    /// Points to the current item in the long_queue
    pub queue_pos: usize,
    /// Should consist of every item added to the queue
    pub long_queue: Vec<QueuedTrack>,
    /// Should consist of the current and next track in the queue
    pub cached: Vec<CachedAudio>,
}

impl QueuedAudio {
    pub fn new(cap: usize) -> QueuedAudio {
        let ring = ringbuf::HeapRb::new(cap);
        QueuedAudio {
            audio_ring: ring,
            queue_pos: 0,
            long_queue: vec![],
            cached: vec![],
        }
    }
    fn read_ring_buf(self) {
        let (_, consumer) = self.audio_ring.split();
    }

    pub fn decode(&mut self, sample_rate: u32) -> Result<String, String> {
        let track = match self.long_queue.get(self.queue_pos) {
            None => return Err("Failed to decode track".to_string()),
            Some(a) => a,
        };

        let mut hint = Hint::new();
        let file = match File::open(track.path_buf.to_str().unwrap().to_string()) {
            Ok(a) => a,
            Err(_) => return Err("Failed to read file".to_string()),
        };

        if let Some(extention) = track.path_buf.extension() {
            hint.with_extension(extention.to_str().unwrap());
        };

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut reader = match symphonia::default::get_probe().probe(
            &hint,
            mss,
            Default::default(),
            Default::default(),
        ) {
            Ok(a) => a,
            Err(_) => return Err("Failed to open reader".to_string()),
        };

        let track = match reader.default_track(TrackType::Audio) {
            None => return Err("No suitable track found in thing".to_string()),
            Some(track) => track,
        };

        let mut codec = match track.codec_params.as_ref() {
            None => return Err("Track lacks codec parameters".to_string()),
            Some(a) => match a {
                CodecParameters::Audio(a) => a.clone(),
                _ => return Err("This is not an audio track".to_string()),
            },
        };

        let mut decoder = match get_codecs().make_audio_decoder(&codec, &Default::default()) {
            Ok(a) => a,
            Err(_) => return Err("Decoder failed".to_string()),
        };

        let track_id = track.id;
        let channels = codec.channels.expect("Failed to get channels");

        let mut resamp = Resamplifier::new(
            codec.sample_rate.expect("There is no sample rate") as usize,
            sample_rate as usize,
            channels,
        );

        while let Some(packet) = reader.next_packet().expect("Failed") {
            if packet.track_id != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(a) => {
                    if resamp.chunk_size.is_none() {
                        resamp.chunk_size = Some(a.capacity())
                    }

                    let out = Box::from(Vec::with_capacity(a.capacity()));
                    resamp.resample(a, out);
                    continue;
                }
                Err(_) => return Err("FAILED".to_string()),
            }
        }

        return Ok("12".to_string());
    }
}
