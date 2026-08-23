use rodio::Sink;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum LoopState {
    LoopingTrack,
    LoopingQueue,
    NotLooping,
    RandomShuffle,
}

#[derive(Clone)]
pub struct AudioOutput {
    pub loop_state: LoopState,
    pub sink: Arc<Sink>,
    pub mixer: Arc<rodio::stream::OutputStream>,
}

impl AudioOutput {
    pub fn new(volume: f32) -> AudioOutput {
        let mixer =
            rodio::OutputStreamBuilder::open_default_stream().expect("Failed to open stream");
        let sink = rodio::Sink::connect_new(mixer.mixer());

        sink.set_volume(volume / 100.0);

        AudioOutput {
            loop_state: LoopState::NotLooping,
            sink: Arc::new(sink),
            mixer: Arc::new(mixer),
        }
    }
}
