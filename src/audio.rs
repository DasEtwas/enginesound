use crate::gen::Generator;
use parking_lot::RwLock;
use sdl2::{
    self,
    audio::{AudioCallback, AudioDevice, AudioSpecDesired},
};
use std::sync::Arc;

pub const GENERATOR_BUFFER_SIZE: usize = 256;
pub const GENERATOR_CHANNEL_SIZE: usize = 6;

pub struct Audio {
    /// dropping this stops the stream
    #[allow(unused)]
    player: AudioDevice<StreamingPlayer>,
}

/// starts audio streaming to an audio device and also steps the generator with a fixed buffer of size `GENERATOR_BUFFER_SIZE`
pub fn init(
    gen: Arc<RwLock<Generator>>,
    sample_rate: u32,
) -> Result<(Audio, crossbeam::Receiver<Vec<f32>>), String> {
    let sdl_context = sdl2::init()?;
    let audio_subsystem = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(sample_rate as i32),
        channels: Some(1),
        samples: Some(crate::SAMPLES_PER_CALLBACK as u16),
    };

    let (generator_sender, device_receiver) = crossbeam::channel::bounded(GENERATOR_CHANNEL_SIZE);
    let (generator_fft_sender, fft_receiver) = crossbeam::channel::bounded(GENERATOR_CHANNEL_SIZE);

    let out_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        if sample_rate == spec.freq as u32 {
            StreamingPlayer {
                stream: ExactStreamer::new(GENERATOR_BUFFER_SIZE, device_receiver),
            }
        } else {
            panic!(
                "Sample rate {} is not provided by the audio system",
                sample_rate
            );
        }
    })?;

    out_device.resume();

    let spec = out_device.spec();

    println!(
        "Audio driver: {:?}\nAudioSpecDesired: Channels: {:?}, Samplerate: {:?}, Samples: {:?}\nActual spec     : Channels: {:?}, Samplerate: {:?}, Samples: {:?}",
        out_device.subsystem().current_audio_driver(),
        desired_spec.channels,
        desired_spec.freq,
        desired_spec.samples,
        spec.channels,
        spec.freq,
        spec.samples
    );

    std::thread::spawn({
        move || {
            let mut buf = [0.0f32; GENERATOR_BUFFER_SIZE];

            loop {
                // contains lock
                {
                    gen.write().generate(&mut buf);
                }

                let _ = generator_fft_sender.try_send(buf.to_vec());

                if generator_sender.send(buf.to_vec()).is_err() {
                    break;
                }
            }
        }
    });

    Ok((Audio { player: out_device }, fft_receiver))
}

/// Used as a kind of `BufReader` for input from a `Receiver<Vec<T>>` to read an exact number of `T`s by buffering and not peeking in the channel
pub struct ExactStreamer<T> {
    /// stores data if the callback's slice's size is not a multiple of `GENERATOR_BUFFER_SIZE`
    remainder: Vec<T>,
    remainder_len: usize,
    /// receives audio from the worker thread
    receiver: crossbeam::Receiver<Vec<T>>,
}

impl<T> ExactStreamer<T>
where
    T: Copy + Default,
{
    pub fn new(
        remainder_buffer_size: usize,
        receiver: crossbeam::Receiver<Vec<T>>,
    ) -> ExactStreamer<T> {
        ExactStreamer {
            remainder: vec![T::default(); remainder_buffer_size],
            remainder_len: 0,
            receiver,
        }
    }

    pub fn fill(&mut self, out: &mut [T]) {
        let mut i = self.remainder_len.min(out.len());

        out[..i].copy_from_slice(&self.remainder[..i]);

        // move old data to index 0 for next read
        self.remainder.copy_within(i..self.remainder_len, 0);
        self.remainder_len -= i;

        while i < out.len() {
            let generated = self
                .receiver
                .recv()
                .expect("Stream channel unexpectedly disconnected");
            if generated.len() > out.len() - i {
                let left = out.len() - i;
                out[i..].copy_from_slice(&generated[..left]);

                self.remainder_len = generated.len() - left;

                let vec_len = self.remainder.len();
                if vec_len < self.remainder_len {
                    self.remainder
                        .extend(std::iter::repeat(T::default()).take(self.remainder_len - vec_len));
                }

                self.remainder[..self.remainder_len].copy_from_slice(&generated[left..]);
                break;
            } else {
                out[i..(i + generated.len())].copy_from_slice(&generated);
                i += generated.len();
            }
        }
    }
}

struct StreamingPlayer {
    stream: ExactStreamer<f32>,
}

impl AudioCallback for StreamingPlayer {
    type Channel = f32;

    /// takes buffered audio from the channel and stores excess data inside `self.samples_remainder`
    fn callback(&mut self, out: &mut [f32]) {
        self.stream.fill(out);
    }
}
