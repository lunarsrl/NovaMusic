use rodio::Player;
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
    pub player: Arc<Player>,
    pub mixer: Arc<rodio::stream::MixerDeviceSink>,
}

impl AudioOutput {
    pub fn new(volume: f32) -> AudioOutput {
        let mixer = rodio::DeviceSinkBuilder::open_default_sink().expect("Failed to open stream");
        let Player = rodio::Player::connect_new(mixer.mixer());

        Player.set_volume(volume / 100.0);

        AudioOutput {
            loop_state: LoopState::NotLooping,
            player: Arc::new(Player),
            mixer: Arc::new(mixer),
        }
    }
}
