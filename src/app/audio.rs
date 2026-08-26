use crate::app::AppTrack;

#[derive(Debug, Clone)]
pub enum LoopState {
    LoopingTrack,
    LoopingQueue,
    NotLooping,
    RandomShuffle,
}

#[derive(Clone)]
pub struct AudioPLayer {
    pub loop_state: LoopState,

    pub song_progress: f64,
    pub song_duration: Option<f64>,
    pub queue: Vec<AppTrack>,
    pub queue_pos: usize,
}

impl AudioPLayer {
    pub fn new(volume: f32) -> AudioPLayer {
        AudioPLayer {
            loop_state: LoopState::NotLooping,
            song_progress: 0.0,
            song_duration: None,
            queue: vec![],
            queue_pos: 0,
        }
    }

    /// Use to clear the queue and associated data
    pub fn clear(&mut self) {
        self.queue_pos = 0;
        self.song_progress = 0.0;
        self.song_duration = None;
        self.queue.clear();
    }
}
