use audioadapter_compat_symphonia::SymphoniaAdapter;
use colored::Colorize;
use rubato::{FixedSync, Indexing, Resampler};
use symphonia::core::audio::{Audio, AudioBuffer, AudioSpec, Channels, GenericAudioBufferRef};

pub struct Resamplifier {
    resampler: rubato::Fft<f32>,
    input: AudioBuffer<f32>,
    output: AudioBuffer<f32>,
    pub chunk_size: Option<usize>,
}

impl Resamplifier {
    pub fn new<'a>(in_rate: usize, out_rate: usize, in_channels: Channels) -> Resamplifier {
        let spec = AudioSpec::new(out_rate as u32, in_channels.clone());
        let out_buf: AudioBuffer<f32> = AudioBuffer::new(spec, 1024);
        let spec = AudioSpec::new(in_rate as u32, in_channels);
        let in_buf: AudioBuffer<f32> = AudioBuffer::new(spec, 1024);

        Resamplifier {
            resampler: rubato::Fft::new(in_rate, out_rate, 1024, 2, FixedSync::Output).unwrap(),
            input: in_buf,
            output: out_buf,
            chunk_size: None,
        }
    }

    pub fn resample(&mut self, in_buffer: GenericAudioBufferRef, out: Box<Vec<f32>>) {
        if self.chunk_size.is_none() {
            std::panic!("Chunk size was not set before trying to decode a packet")
        }

        self.set_input(in_buffer);
        self.output.clear();

        while self.chunk_size.unwrap() <= self.input.frames() {
            let next = self.resampler.output_frames_next();

            log::info!(
                "IN_FRAMES: {}, OUT_FRAMES {}, NEXT: {}",
                self.input.frames(),
                self.output.frames(),
                next
            );

            self.output.grow_capacity(self.output.frames() + next);
            self.output.render_uninit(Some(next));

            let (a, _) = self
                .resampler
                .process_into_buffer(
                    &SymphoniaAdapter::new(&self.input),
                    &mut SymphoniaAdapter::new_mut(&mut self.output),
                    None,
                )
                .expect("Buffer failed");

            self.input.shift(a);
        }

        self.input.clear();

        self.output.copy_to_slice_interleaved(*out);
    }

    pub fn set_input(&mut self, in_buffer: GenericAudioBufferRef) {
        let frame_count = in_buffer.frames();

        if self.input.capacity() < frame_count {
            self.input.grow_capacity(frame_count)
        }

        self.input.render_uninit(Some(in_buffer.frames()));

        log::info!("INPUT");
        in_buffer.copy_to(&mut self.input);
        log::info!("{}", "SUCCESS".green());
    }
}
