use std::time::{Duration, Instant};

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

    std::thread::sleep(Duration::from_secs(100));

    Ok(())
}

const I16_MAX: f32 = std::i16::MAX as f32;

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
        self.gen.lock().generate(out);
        self.lastnanos = Instant::now().duration_since(start_time).as_nanos();
        self.nanos += self.lastnanos;
        self.counter += out.len() as u32;

        // print avg computation time every 10 seconds
        if self.counter % (crate::SAMPLE_RATE / out.len() as u32 * 2) == 0 {
            println!(
                "{:.5} us/sample\t| {:.1}% duty",
                self.nanos as f64 / self.counter as f64 * 1E-3,
                self.lastnanos as f64 / out.len() as f64 / (1E9 / crate::SAMPLE_RATE as f64) * 100.0
            );
        }
    }
}
