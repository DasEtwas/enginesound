use crate::exactstreamer::ExactStreamer;
use num_complex::Complex32;
use num_traits::identities::Zero;
use rustfft::FFT;
use std::time::Instant;

pub struct FFTStreamer {
    size: usize,
    stream: ExactStreamer<f32>,
    sender: crossbeam_channel::Sender<Vec<f32>>,
}

impl FFTStreamer {
    pub fn new(
        size: usize,
        stream: ExactStreamer<f32>,
        sender: crossbeam_channel::Sender<Vec<f32>>,
    ) -> Self {
        FFTStreamer {
            size,
            stream,
            sender,
        }
    }

    pub fn run(&mut self) {
        let mut buf = vec![0.0f32; self.size];
        let mut complex_buf = vec![Complex32::zero(); self.size];
        let mut complex_buf2 = vec![Complex32::zero(); self.size];

        let mut frequencies = vec![0.0; self.size];
        let mut last_frequencies = vec![0.0; self.size];
        let mut last_time = Instant::now();

        let fft = rustfft::algorithm::Radix4::new(self.size, false);

        loop {
            if self.stream.fill(&mut buf).is_err() {
                break;
            }

            let window_fac = std::f32::consts::PI * 2.0 / self.size as f32;
            complex_buf.clear();
            complex_buf.extend(buf.iter().enumerate().map(|(i, sample)| {
                Complex32::new(*sample * (0.54 - 0.46 * (i as f32 * window_fac).cos()), 0.0)
            }));

            fft.process(&mut complex_buf, &mut complex_buf2);

            frequencies
                .iter_mut()
                .zip(complex_buf2.iter().map(|complex| complex.norm()))
                .for_each(|(old, new)| *old = new);

            let fac = 0.00005f32.powf(last_time.elapsed().as_secs_f32());
            last_time = Instant::now();
            last_frequencies
                .iter_mut()
                .zip(frequencies.iter())
                .for_each(|(old, new)| {
                    //(coefficient after one second).powf(time))
                    *old *= fac;
                    *old = old.max(*new);
                });

            if self
                .sender
                .send(
                    last_frequencies
                        .iter()
                        .map(|x| (((x * 0.008).exp() - 1.0) * 0.7).powf(0.5) * 2.0)
                        .collect::<Vec<f32>>(),
                )
                .is_err()
            {
                break;
            }
        }
    }
}
