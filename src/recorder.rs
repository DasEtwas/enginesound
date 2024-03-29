use hound::{SampleFormat, WavSpec};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::{
    fs::File,
    io::BufWriter,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct Recorder {
    /// recorded samples since creation
    len: usize,
    sender: crossbeam_channel::Sender<Vec<f32>>,
    running: Arc<AtomicBool>,
    block_lock: Arc<Mutex<()>>,
}

impl Recorder {
    pub fn new(file: PathBuf, sample_rate: u32) -> Recorder {
        let (send, recv) = crossbeam_channel::unbounded();

        let ret = Recorder {
            len: 0,
            sender: send,
            running: Arc::new(AtomicBool::new(true)),
            block_lock: Arc::new(Mutex::new(())),
        };
        ret.start(recv, file, sample_rate);
        ret
    }

    fn start(&self, recv: crossbeam_channel::Receiver<Vec<f32>>, file: PathBuf, sample_rate: u32) {
        std::thread::spawn({
            let running = self.running.clone();
            let block_lock = self.block_lock.clone();
            move || {
                let lock = block_lock.lock();

                let mut wav_writer = match hound::WavWriter::new(
                    BufWriter::new(File::create(&file).unwrap_or_else(|e| {
                        panic!("Failed to create/open a file for writing the WAV: {}", e)
                    })),
                    WavSpec {
                        channels: 1,
                        sample_rate,
                        bits_per_sample: 32,
                        sample_format: SampleFormat::Float,
                    },
                ) {
                    Ok(wav_writer) => wav_writer,
                    Err(e) => panic!("Failed to create a WavWriter: {}", e),
                };

                while running.load(Ordering::Relaxed) {
                    match recv.recv_timeout(Duration::from_secs(4)) {
                        Ok(samples) => {
                            samples
                                .iter()
                                .for_each(|sample| wav_writer.write_sample(*sample).unwrap());
                        }
                        Err(_) => break,
                    }
                }

                println!("Stopped recording, finishing writing WAV..");

                while let Ok(samples) = recv.try_recv() {
                    samples
                        .iter()
                        .for_each(|sample| wav_writer.write_sample(*sample).unwrap());
                }

                wav_writer.flush().unwrap();

                println!(
                    "Done writing WAV to File \"{}\" (wrote {:.3} sec)",
                    file.to_str().unwrap_or("<invalid UTF-8>"),
                    wav_writer.len() as f32 / sample_rate as f32
                );

                // keeping lock in scope explicitly
                std::mem::drop(lock);
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
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn stop_wait(&self) {
        self.running.store(false, Ordering::Relaxed);

        while !self.sender.is_empty() {}

        let _ = self.block_lock.lock();
    }
}
