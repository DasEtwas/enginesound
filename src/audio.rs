use std::time::Instant;

use crate::gen::Generator;
use parking_lot::Mutex;
use sdl2::{self,
           audio::{AudioCallback, AudioDevice, AudioSpecDesired}};
use std::sync::{atomic::Ordering, Arc};

pub struct Audio {
    /// only kept to keep the sound system alive
    #[allow(unused)]
    player: AudioDevice<StreamingPlayer>,
}

pub fn init(gen: Arc<Mutex<Generator>>, sample_rate: u32) -> Result<Audio, String> {
    let sdl_context = sdl2::init()?;
    let audio_subsystem = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(sample_rate as i32), channels: Some(1), samples: None
    };

    let out_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        if sample_rate == spec.freq as u32 {
            StreamingPlayer {
                gen,
                counter: 0,
                nanos: 0,
                lastnanos: 0,
            }
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

    Ok(Audio {
        player: out_device
    })
}

struct StreamingPlayer {
    gen:       Arc<Mutex<Generator>>,
    counter:   u32,
    nanos:     u128,
    lastnanos: u128,
}

impl AudioCallback for StreamingPlayer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let start_time = Instant::now();
        let mut gen = self.gen.lock();
        gen.generate(out);

        self.lastnanos = Instant::now().duration_since(start_time).as_nanos();
        self.nanos += self.lastnanos;
        self.counter += out.len() as u32;

        gen.sampler_duty.store((self.lastnanos as f32 / out.len() as f32 / (1E9 / crate::SAMPLE_RATE as f32)).to_bits(), Ordering::Relaxed);
    }
}
