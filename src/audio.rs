use crate::exactstreamer::ExactStreamer;
use crate::gen::Generator;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::{BufferSize, Host, SampleRate, Stream, StreamConfig};
use parking_lot::RwLock;
use std::sync::Arc;

pub const GENERATOR_BUFFER_SIZE: usize = 256;
pub const GENERATOR_CHANNEL_SIZE: usize = 6;

pub struct Audio {
    #[allow(dead_code)]
    speaker_stream: Stream,
}

/// starts audio streaming to an audio device and also steps the generator with a fixed buffer of size `GENERATOR_BUFFER_SIZE`
pub fn init(
    gen: Arc<RwLock<Generator>>,
    sample_rate: u32,
) -> Result<(Audio, crossbeam_channel::Receiver<Vec<f32>>), String> {
    let (generator_sender, device_receiver) = crossbeam_channel::bounded(GENERATOR_CHANNEL_SIZE);
    let (generator_fft_sender, fft_receiver) = crossbeam_channel::bounded(GENERATOR_CHANNEL_SIZE);

    let host: Host = cpal::default_host();
    let speaker = host
        .default_output_device()
        .ok_or_else(|| "Failed to get default audio output device".to_string())?;

    println!(
        "Audio driver: {:?}\nSamplerate: {} Hz",
        host.id(),
        sample_rate
    );

    println!("Audio output device: {}", speaker.name().unwrap());

    let stream_config = StreamConfig {
        sample_rate: SampleRate(sample_rate),
        channels: 1,
        buffer_size: BufferSize::Default,
    };

    println!("Audio output format: {:?}", stream_config);

    let speaker_stream = speaker
        .build_output_stream::<f32, _, _>(
            &stream_config,
            {
                let mut stream = ExactStreamer::new(GENERATOR_BUFFER_SIZE, device_receiver);

                move |data, _info| {
                    let _ = stream.fill(data);
                }
            },
            move |e| {
                println!("== An error occurred during audio playback: {:?}", e);
            },
        )
        .expect("Failed to build audio output stream");

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

    Ok((Audio { speaker_stream }, fft_receiver))
}
