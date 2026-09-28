use audioadapter_compat_symphonia::SymphoniaAdapter;
use colored::Colorize;
use rubato::{FixedSync, Indexing, Resampler};
use rusqlite::fallible_iterator::FallibleIterator;
use std::time::Duration;
use symphonia::core::audio::{
    Audio, AudioBuffer, AudioBytes, AudioSpec, Channels, GenericAudioBufferRef,
};

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

        //out
        let spec = AudioSpec::new(out_rate as u32, in_channels.clone());
        let out_buf: AudioBuffer<f32> = AudioBuffer::new(spec, resampler.output_frames_max());
        //in
        let spec = AudioSpec::new(in_rate as u32, in_channels.clone());
        let in_buf: AudioBuffer<f32> = AudioBuffer::new(spec, in_chunk);

        Resamplifier {
            resampler,
            input: in_buf,
            output: out_buf,
            chunk_size: in_chunk,
        }
    }

    pub fn resample(&mut self, audio: GenericAudioBufferRef, out: &mut Vec<f32>) {
        self.set_input(audio);
        self.output.clear();
        log::info!(
            "{}: {}",
            "[Resampling]".blue(),
            format!(
                "Chunk Size: {}; Input Frames #: {}",
                self.chunk_size,
                self.input.frames()
            )
        );

        while self.chunk_size <= self.input.frames() {
            let next = self.resampler.output_frames_next();
            log::info!(
                "{} Next: {} Current: {}",
                "[Resampling]".blue(),
                next,
                self.output.frames()
            );
            self.output.grow_capacity(self.output.frames() + next);
            log::info!(
                "{} Capacity grew to: {}",
                "[Resampling]".blue(),
                self.output.frames() + next
            );
            self.output.render_uninit(Some(next));
            log::info!(
                "{} New frame count: {}",
                "[Resampling]".blue(),
                self.output.frames()
            );

            let (a, _) = self
                .resampler
                .process_into_buffer(
                    &SymphoniaAdapter::new(&self.input),
                    &mut SymphoniaAdapter::new_mut(&mut self.output),
                    None,
                )
                .expect("Buffer failed");

            log::info!(
                "{}: {}",
                "[Resampling]".blue(),
                format!("Shifting data: {}", a)
            );

            self.input.shift(a);
        }

        log::info!(
            "{}: Finished Chunk! {}",
            "[Resampling]".blue(),
            self.output.frames()
        );

        self.output.copy_to_vec_interleaved(out);
    }

    fn set_input(&mut self, audio: GenericAudioBufferRef) {
        let frame_count = audio.frames() + self.input.frames();

        self.input.grow_capacity(frame_count);
        self.input.render_uninit(Some(frame_count));

        audio.copy_to(&mut self.input);

        log::info!(
            "{}: Input: {}, Audio: {}",
            "[Resampling]".blue(),
            self.input.frames(),
            audio.frames()
        );
    }
}
