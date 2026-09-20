use crate::app::audio::AudioPlayer;

pub struct AudioBuffer {
    pub ring: rb::SpscRb<f32>,
}

impl AudioBuffer {
    pub fn new(player: &mut AudioPlayer) -> AudioBuffer {
        let ring = AudioBuffer {
            ring: rb::SpscRb::new(((player.stream_config.sample_rate * 5) as usize) as usize),
        };
        return ring;
    }
}
