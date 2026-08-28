use crate::app::audio::LoopState;

pub struct PlayerState {
    pub loop_state: LoopState,
    pub song_progress: f64,
    pub song_duration: Option<f64>,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            loop_state: LoopState::NotLooping,
            song_progress: 0.0,
            song_duration: None,
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
