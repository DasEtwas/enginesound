use crate::exactstreamer::ExactStreamer;
use crate::gen::Generator;
use cpal::traits::DeviceTrait;
use cpal::traits::EventLoopTrait;
use cpal::traits::HostTrait;
use cpal::{Format, Host, SampleRate, StreamData, UnknownTypeOutputBuffer};
use parking_lot::RwLock;
use std::sync::Arc;

pub const GENERATOR_BUFFER_SIZE: usize = 256;
pub const GENERATOR_CHANNEL_SIZE: usize = 6;

pub struct Audio {}

/// starts audio streaming to an audio device and also steps the generator with a fixed buffer of size `GENERATOR_BUFFER_SIZE`
pub fn init(
    gen: Arc<RwLock<Generator>>,
    sample_rate: u32,
) -> Result<(Audio, crossbeam::Receiver<Vec<f32>>), String> {
    let (generator_sender, device_receiver) = crossbeam::channel::bounded(GENERATOR_CHANNEL_SIZE);
    let (generator_fft_sender, fft_receiver) = crossbeam::channel::bounded(GENERATOR_CHANNEL_SIZE);

    let host: Host = cpal::default_host();
    let event_loop = host.event_loop();
    let speaker = host
        .default_output_device()
        .ok_or_else(|| "Failed to get default audio output device".to_string())?;

    println!("Audio output device: {}", speaker.name().unwrap());

    let format = speaker
        .default_output_format()
        .expect("Failed to get default audio device's default output format");

    println!("Audio output format: {:?}", format.data_type);

    let speaker_stream_id = event_loop
        .build_output_stream(
            &speaker,
            &Format {
                sample_rate: SampleRate(sample_rate),
                channels: 1,
                data_type: format.data_type,
            },
        )
        .expect("Failed to build audio output stream");

    event_loop.play_stream(speaker_stream_id).unwrap();

    std::thread::spawn({
        move || {
            let mut stream = ExactStreamer::new(GENERATOR_BUFFER_SIZE, device_receiver);

            let mut buf = [0.0; 8192];

            event_loop.run(move |_stream_id, data| match data {
                Ok(StreamData::Output { buffer }) => match buffer {
                    UnknownTypeOutputBuffer::F32(mut data) => {
                        let _ = stream.fill(&mut data);
                    }
                    UnknownTypeOutputBuffer::U16(mut data) => {
                        let _ = stream.fill(&mut buf[..data.len()]);
                        data.iter_mut().zip(buf.iter()).for_each(|(a, &b)| {
                            *a = if b > 1.0 {
                                std::u16::MAX
                            } else if b < -1.0 {
                                0
                            } else {
                                (((b + 1.0) * std::u16::MAX as f32) as u32 / 2) as u16
                            }
                        });
                    }
                    UnknownTypeOutputBuffer::I16(mut data) => {
                        let _ = stream.fill(&mut buf[..data.len()]);
                        data.iter_mut().zip(buf.iter()).for_each(|(a, &b)| {
                            *a = (if b > 1.0 {
                                std::u16::MAX
                            } else if b < -1.0 {
                                0
                            } else {
                                (((b + 1.0) * std::u16::MAX as f32) as u32 / 2) as u16
                            } as i32
                                + std::u16::MIN as i32) as i16
                        });
                    }
                },
                Err(e) => {
                    println!("== An error occurred: {}", e);
                }
                _ => (),
            });
        }
    });

    println!(
        "Audio driver: {:?}\nSamplerate: {} Hz",
        host.id(),
        sample_rate
    );

    std::thread::spawn({
        move || {
            let mut buf = [0.0f32; GENERATOR_BUFFER_SIZE];

            loop {
                // contains lock guard
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

    Ok((Audio {}, fft_receiver))
}
