use hound::{SampleFormat, WavSpec};
use std::{fs::File,
          sync::{atomic::{AtomicBool, Ordering},
                 Arc}};

pub struct Recorder {
    /// recorded samples since creation
    len: usize,
    sender: crossbeam::Sender<Vec<f32>>,
    running: Arc<AtomicBool>,
}

impl Recorder {
    pub fn new(filename: String) -> Recorder {
        let (send, recv) = crossbeam::unbounded();

        let ret = Recorder {
            len: 0, sender: send, running: Arc::new(AtomicBool::new(true))
        };
        ret.start(recv, filename);
        ret
    }

    fn start(&self, recv: crossbeam::Receiver<Vec<f32>>, filename: String) {
        std::thread::spawn({
            let running = self.running.clone();
            move || {
                let mut wav_writer = match hound::WavWriter::new(
                    File::create(filename.as_str()).unwrap_or_else(|e| panic!("Failed to create/open a file for writing the WAV: {}", e)),
                    WavSpec {
                        channels: 1, sample_rate: crate::SAMPLE_RATE, bits_per_sample: 16, sample_format: SampleFormat::Int
                    },
                ) {
                    Ok(wav_writer) => wav_writer,
                    Err(e) => panic!("Failed to create a WavWriter: {}", e),
                };

                while running.load(Ordering::Relaxed) {
                    match recv.recv() {
                        Ok(samples) => {
                            samples.iter().for_each(|sample| wav_writer.write_sample((*sample * std::i16::MAX as f32) as i16).unwrap());
                        },
                        Err(_) => break,
                    }
                }

                println!("Stopped recording, finishing writing WAV..");

                while let Ok(samples) = recv.try_recv() {
                    samples.iter().for_each(|sample| wav_writer.write_sample((*sample * std::i16::MAX as f32) as i16).unwrap());
                }

                wav_writer.flush().unwrap();

                println!("Done writing WAV to File \"{}\" ({:.3} sec)", filename, wav_writer.len() as f32 / crate::SAMPLE_RATE as f32);
            }
        });
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn record(&mut self, samples: Vec<f32>) {
        if self.is_running() {
            self.len += samples.len();
            self.sender.send(samples).unwrap();
        }
    }

    pub fn get_len(&self) -> usize {
        self.len
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed)
    }
}
