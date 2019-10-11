use crate::audio::ExactStreamer;
use num_complex::Complex32;
use num_traits::identities::Zero;
use rustfft::FFT;

pub struct FFTStreamer {
    size: usize,
    stream: ExactStreamer<f32>,
    sender: crossbeam::Sender<Vec<f32>>,
}

impl FFTStreamer {
    pub fn new(
        size: usize,
        stream: ExactStreamer<f32>,
        sender: crossbeam::Sender<Vec<f32>>,
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
        loop {
            self.stream.fill(&mut buf);

            complex_buf.clear();
            complex_buf.extend(buf.iter().map(|sample| Complex32::new(*sample, 0.0)));

            rustfft::algorithm::Radix4::new(self.size, false)
                .process(&mut complex_buf, &mut complex_buf2);
            /* fft(&mut complex_buf[..]);
            complex_buf2.copy_from_slice(&complex_buf);*/

            if self
                .sender
                .send(
                    complex_buf2
                        .iter()
                        .map(|complex| complex.norm())
                        .collect::<Vec<f32>>(),
                )
                .is_err()
            {
                break;
            }
        }
    }
}

/*
/// Radix-2 DIF FFT
/// writes output into `input`
#[inline]
pub fn fft(input: &mut [Complex32]) {
    assert_eq!(2u32.pow(log2ui(input.len() as u32)), input.len() as u32);
    assert!(input.len() > 1);

    let mut output = vec![Complex32::zero(); input.len()];
    fft_recurse(input, &mut output);
    reorder(&output, input);
}

#[inline]
fn fft_recurse(input: &mut [Complex32], output: &mut [Complex32]) {
    if input.len() == 2 {
        output[0] = input[0] + input[1];
        output[1] = input[0] - input[1];
    } else {
        let half_len = input.len() / 2;

        for i in 0..half_len {
            let inputi = input[i];
            let inputi_half = input[i + half_len];
            input[i] = inputi + inputi_half;
            input[i + half_len] = inputi - inputi_half;
        }

        fft_recurse(&mut input[..half_len], &mut output[..half_len]);

        fft_recurse(&mut input[half_len..], &mut output[half_len..]);
    }
}

fn reorder(input: &[Complex32], reordered_buf: &mut [Complex32]) {
    for i in 0..input.len() {
        reordered_buf[reverse_bits(i as u32, log2ui(input.len() as u32) as usize) as usize] =
            input[i];
    }
}

fn reverse_bits(value: u32, count: usize) -> u32 {
    (0..count).fold(0u32, |acc, i| (acc << 1) | (value >> i) & 1)
}

/// Returns floor(log2(`n`))
#[inline]
pub const fn log2ui(n: u32) -> u32 {
    (62 - (n.leading_zeros() << 1)) >> 1
}*/
