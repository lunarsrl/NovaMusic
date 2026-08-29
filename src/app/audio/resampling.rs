use audioadapter_compat_symphonia::SymphoniaAdapter;
use colored::Colorize;
use rubato::{FixedSync, Indexing, Resampler};
use rusqlite::fallible_iterator::FallibleIterator;
use symphonia::core::audio::{Audio, AudioBuffer, AudioSpec, Channels, GenericAudioBufferRef};

pub struct Resamplifier {
    resampler: rubato::Fft<f32>,
    input: AudioBuffer<f32>,
    output: AudioBuffer<f32>,
    pub chunk_size: usize,
}

impl Resamplifier {
    pub fn new<'a>(
        in_rate: usize,
        out_rate: usize,
        in_channels: Channels,
        in_chunk: usize,
    ) -> Resamplifier {
        let resampler = rubato::Fft::new(
            in_rate,
            out_rate,
            in_chunk,
            in_channels.count(),
            FixedSync::Input,
        )
        .unwrap();

        let spec = AudioSpec::new(out_rate as u32, in_channels.clone());
        let out_buf: AudioBuffer<f32> = AudioBuffer::new(spec, resampler.output_frames_max());
        let spec = AudioSpec::new(in_rate as u32, in_channels.clone());
        let in_buf: AudioBuffer<f32> = AudioBuffer::new(spec, in_chunk);

        Resamplifier {
            resampler,
            input: in_buf,
            output: out_buf,
            chunk_size: in_chunk,
        }
    }

    pub fn resample(&mut self, in_buffer: GenericAudioBufferRef, out: &mut Vec<f32>) {
        self.set_input(in_buffer);
        self.output.clear();

        while self.chunk_size <= self.input.frames() {
            let next = self.resampler.output_frames_next();

            log::info!(
                "IN_FRAMES: {}, IN_NEXT {}, OUT_NEXT: {}",
                self.input.frames(),
                self.resampler.input_frames_next(),
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

            log::info!("Shifting [{}]!", a.to_string().yellow());

            self.input.shift(a);
        }

        self.output.copy_to_vec_interleaved(out);
        self.input.clear();
    }

    pub fn set_input(&mut self, in_buffer: GenericAudioBufferRef) {
        let frame_count = in_buffer.frames();

        self.input.grow_capacity(frame_count);

        self.input.render_uninit(Some(frame_count));

        log::info!("INPUT");
        in_buffer.copy_to(&mut self.input);
        log::info!("{}", "SUCCESS".green());
    }
}
