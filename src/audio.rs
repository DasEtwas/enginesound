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
pub fn init(gen: Arc<RwLock<Generator>>, sample_rate: u32) -> Result<Audio, String> {
    let sdl_context = sdl2::init()?;
    let audio_subsystem = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired { freq: Some(sample_rate as i32), channels: Some(1), samples: Some(crate::SAMPLES_PER_CALLBACK as u16) };

    let (generator_sender, device_receiver) = crossbeam::channel::bounded(GENERATOR_CHANNEL_SIZE);

    let out_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        if sample_rate == spec.freq as u32 {
            StreamingPlayer { samples_remainder: [0.0f32; GENERATOR_BUFFER_SIZE], samples_remainder_len: 0, audio_receiver: device_receiver }
        } else {
            panic!("Sample rate {} is not provided by the audio system", sample_rate);
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

                if generator_sender.send(buf).is_err() {
                    break;
                }
            }
        }
    });

    Ok(Audio { player: out_device })
}

struct StreamingPlayer {
    /// stores data if the callback's slice's size is not a multiple of `GENERATOR_BUFFER_SIZE`
    samples_remainder: [f32; GENERATOR_BUFFER_SIZE],
    samples_remainder_len: usize,
    /// receives audio from the worker thread
    audio_receiver: crossbeam::Receiver<[f32; GENERATOR_BUFFER_SIZE]>,
}

impl AudioCallback for StreamingPlayer {
    type Channel = f32;

    /// takes buffered audio from the channel and stores excess data inside `self.samples_remainder`
    fn callback(&mut self, out: &mut [f32]) {
        let mut i = self.samples_remainder_len.min(out.len());

        out[..i].copy_from_slice(&self.samples_remainder[..i]);

        // move old data to index 0 for next read
        self.samples_remainder.copy_within(i..self.samples_remainder_len, 0);
        self.samples_remainder_len -= i;

        while i < out.len() {
            let generated = self.audio_receiver.recv().expect("Audio generator thread unexpectedly disconnected from channel");
            if generated.len() > out.len() - i {
                let left = out.len() - i;
                out[i..].copy_from_slice(&generated[..left]);

                self.samples_remainder_len = generated.len() - left;

                self.samples_remainder[..self.samples_remainder_len].copy_from_slice(&generated[left..]);
                break;
            } else {
                out[i..(i + generated.len())].copy_from_slice(&generated);
                i += generated.len();
            }
        }
    }
}
