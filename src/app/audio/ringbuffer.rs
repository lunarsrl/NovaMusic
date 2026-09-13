pub struct AudioBuffer {
    pub ring: rb::SpscRb<f32>,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32) -> AudioBuffer {
        AudioBuffer {
            ring: rb::SpscRb::new((sample_rate * 5) as usize),
        }
    }
}
