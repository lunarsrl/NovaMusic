use crate::app::audio::LoopState;

pub struct AudioState {
    pub loop_state: LoopState,
    pub song_progress: f64,
    pub song_duration: Option<f64>,
    pub play_pause: bool,
}

impl AudioState {
    pub fn new() -> AudioState {
        AudioState {
            loop_state: LoopState::NotLooping,
            song_progress: 0.0,
            song_duration: None,
            play_pause: false,
        }
    }

    pub fn loop_state_toggle(mut self) {
        match self.loop_state {
            LoopState::LoopingTrack => {
                self.loop_state = LoopState::RandomShuffle;
            }
            LoopState::LoopingQueue => {
                self.loop_state = LoopState::LoopingTrack;
            }
            LoopState::NotLooping => {
                self.loop_state = LoopState::LoopingQueue;
            }
            LoopState::RandomShuffle => self.loop_state = LoopState::NotLooping,
        }
    }
}
