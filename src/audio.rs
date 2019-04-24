use std::time::Duration;

use crate::gen::Generator;
use parking_lot::Mutex;
use sdl2::{self,
           audio::{AudioCallback, AudioSpecDesired}};
use std::sync::Arc;

pub fn init(gen: Arc<Mutex<Generator>>, sample_rate: u32) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let audio_subsystem = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(sample_rate as i32), channels: Some(1), samples: None
    };

    let mut out_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        if sample_rate == spec.freq as u32 {
            StreamingPlayer {
                gen,
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

    std::thread::sleep(Duration::from_secs(100));

    Ok(())
}

const I16_MAX: f32 = std::i16::MAX as f32;

struct StreamingPlayer {
    gen: Arc<Mutex<Generator>>,
}

impl AudioCallback for StreamingPlayer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        self.gen.lock().generate(out);
    }
}
