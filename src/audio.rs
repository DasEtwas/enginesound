use crate::exactstreamer::ExactStreamer;
use crate::gen::Generator;
use cpal::traits::DeviceTrait;
use cpal::traits::EventLoopTrait;
use cpal::traits::HostTrait;
use cpal::{
    Format, Host, SampleFormat, SampleRate, StreamData, UnknownTypeInputBuffer,
    UnknownTypeOutputBuffer,
};
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
        .ok_or(format!("Failed to get default audio output device"))?;

    println!("== Audio ouput: {}", speaker.name().unwrap());

    let speaker_stream_id = event_loop
        .build_output_stream(
            &speaker,
            &Format {
                sample_rate: SampleRate(sample_rate),
                channels: 1,
                data_type: SampleFormat::F32,
            },
        )
        .expect("Failed to build audio output stream");

    event_loop.play_stream(speaker_stream_id.clone()).unwrap();

    std::thread::spawn({
        move || {
            let mut stream = ExactStreamer::new(GENERATOR_BUFFER_SIZE, device_receiver);

            event_loop.run(move |stream_id, data| match data {
                Ok(StreamData::Output {
                    buffer: UnknownTypeOutputBuffer::F32(mut data),
                }) => {
                    stream.fill(&mut data);
                }
                Err(e) => {
                    println!("== An error occurred: {}", e);
                    return;
                }
                _ => return,
            });
        }
    });

    println!(
        "Audio driver: {:?}\nSamplerate: {:?}",
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
