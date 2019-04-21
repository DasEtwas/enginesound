use std::{io::{Read, Write}, sync::{atomic::{AtomicUsize, Ordering}, Arc, Mutex}};

use byteorder::{BigEndian, ByteOrder};
use sdl2::{self, audio::{AudioCallback, AudioDevice, AudioSpecDesired}, AudioSubsystem};

pub fn init() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let audio_subsystem = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(44100), channels: Some(1), samples: None
    };

    let out_device = audio_subsystem.open_playback(None, &desired_spec, |_| StreamingPlayer {}).unwrap();

    out_device.resume();

    println!(
        "Audio driver: {:?}\nAudioSpecDesired: (Channels: {:?}, Samplerate: {:?}, Samples: {:?})",
        out_device.subsystem().current_audio_driver(),
        desired_spec.channels,
        desired_spec.freq,
        desired_spec.samples
    );
    Ok(())
}

struct StreamingPlayer {}

impl AudioCallback for StreamingPlayer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {

    }
}
