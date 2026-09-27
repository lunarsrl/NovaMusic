use crate::app::audio::LoopState;
use cosmic::widget::icon::Named;

pub struct AudioState {
    pub loop_state: LoopState,
    pub song_progress: i64,
    pub song_duration: Option<i64>,
    pub play_pause: bool,
    pub is_decoding: bool,
}

impl AudioState {
    pub fn new() -> AudioState {
        AudioState {
            loop_state: LoopState::NotLooping,
            song_progress: 0,
            song_duration: None,
            play_pause: false,
            is_decoding: false,
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

    pub fn play_pause_toggle(&mut self) -> bool {
        self.play_pause = !self.play_pause;
        !self.play_pause
    }

    pub fn play_pause_icons<'a>(&self) -> Named {
        match self.play_pause {
            false => cosmic::widget::icon::from_name("media-playback-start-symbolic"),
            true => cosmic::widget::icon::from_name("media-playback-pause-symbolic"),
        }
    }

    pub fn loop_state_icons<'a>(self) -> Named {
        match self.loop_state {
            LoopState::LoopingTrack => {
                cosmic::widget::icon::from_name("media-playlist-repeat-song-symbolic")
            }
            LoopState::LoopingQueue => {
                cosmic::widget::icon::from_name("media-playlist-repeat-symbolic")
            }
            LoopState::NotLooping => {
                cosmic::widget::icon::from_name("media-playlist-consecutive-symbolic")
            }
            LoopState::RandomShuffle => {
                cosmic::widget::icon::from_name("media-playlist-shuffle-symbolic")
            }
        }
    }
}
