use lazy_static::*;
use sdl2::audio::AudioSpec;
use std::sync::atomic::{AtomicU32, Ordering};

use simdeez::{avx2::*, scalar::*, sse2::*, sse41::*, *};

pub const PI: f64 = std::f64::consts::PI;
pub const PI2: f64 = 2.0f64 * std::f64::consts::PI;
pub const PI_2: f64 = std::f64::consts::PI / 2.0f64;
pub const PI_4: f64 = std::f64::consts::PI / 4.0f64;
pub const PI2F: f32 = PI2 as f32;

const SINTABLE_SIZE: usize = 128;
const SINTABLE_SIZE_4: usize = SINTABLE_SIZE / 4;
const SINTABLE_SIZE__2PI: f32 = SINTABLE_SIZE as f32 / PI2 as f32;

lazy_static! {
    static ref SINTABLE: [f32; SINTABLE_SIZE] = {
        let mut table = [0f32; SINTABLE_SIZE];
        let SINTABLE_SIZEf = SINTABLE_SIZE as f64;
        for i in 0..SINTABLE_SIZE {
            table[i] = (i as f64 * PI2 / SINTABLE_SIZEf).sin() as f32;
        }
        table
    };
}

#[inline]
pub fn fast_sin(x: f32) -> f32 {
    let a = x * SINTABLE_SIZE__2PI;
    let b = a as usize;
    let c = a % 1.0;
    SINTABLE[b % SINTABLE_SIZE] * (1.0 - c) + SINTABLE[(b + 1) % SINTABLE_SIZE] * c
    //SINTABLE[((x % PI2F) * SINTABLE_SIZE__2PI) as usize % SINTABLE_SIZE]
}

#[inline]
pub fn fast_cos(x: f32) -> f32 {
    let a = x * SINTABLE_SIZE__2PI;
    let b = a as usize + SINTABLE_SIZE_4;
    let c = a % 1.0;
    SINTABLE[b % SINTABLE_SIZE] * (1.0 - c) + SINTABLE[(b + 1) % SINTABLE_SIZE] * c
    //SINTABLE[(((x % PI2F) * SINTABLE_SIZE__2PI) as usize + SINTABLE_SIZE_4) % SINTABLE_SIZE]
}

// https://www.researchgate.net/profile/Stefano_Delle_Monache/publication/280086598_Physically_informed_car_engine_sound_synthesis_for_virtual_and_augmented_environments/links/55a791bc08aea2222c746724/Physically-informed-car-engine-sound-synthesis-for-virtual-and-augmented-environments.pdf?origin=publication_detail

/// As the combustion is fast, though not perfectly instantaneous, this event is represented as the positive
/// half of a sine wave, shifted at the beginning of the expansion phase and rescaled by a parameter t, which represents
/// the time (relative to the full engine cycle) needed by the fuel to explode
const IGNITION_SCALE: f64 = 0.1;
const VOLUME: f32 = 0.3;

pub struct Generator {
    samples_per_second: u32,
    engine_params:      EngineParameters,
}

pub struct EngineParameters {
    rpm: AtomicU32,
    /// crankshaft position, not normalized
    crankshaft_pos: f32,
    lp: LowPassFilter,
}

impl Generator {
    pub(crate) fn new(samples_per_second: u32) -> Generator {
        Generator {
            samples_per_second,
            engine_params: EngineParameters {
                rpm:            AtomicU32::new(12000.0f32.to_bits()),
                crankshaft_pos: 0.0,
                lp:             LowPassFilter::new(90.0, samples_per_second),
            },
        }
    }

    pub(crate) fn generate(&mut self, buf: &mut [f32]) {
        let len = buf.len() as f32;

        let crankshaft_pos = self.engine_params.crankshaft_pos;
        let samples_per_second = self.samples_per_second as f32 * 120.0;

        let mut i = 1.0;
        let mut ii = 0;
        while ii < buf.len() {
            self.engine_params.crankshaft_pos = (crankshaft_pos + i * self.get_rpm() / samples_per_second) % 1.0;
            let a = self.gen();
            buf[ii] = self.engine_params.lp.filter(a);
            i += 1.0;
            ii += 1;
        }
    }

    fn get_rpm(&self) -> f32 {
        f32::from_bits(self.engine_params.rpm.load(Ordering::Relaxed))
    }

    /// generates one sample worth of data
    fn gen(&mut self) -> f32 {
        let a = self.engine_params.crankshaft_pos * std::f32::consts::PI * 2.0;
        fast_sin(a) * 0.5
    }
}

/*

fn main() {
    let spec = AudioSpecDesired {
        channels: 1, sample_rate: 41000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int
    };

    let duration = 20.0; //seconds

    let onesec = get_length(&spec, 1.0) as f64;
    let delay_samples = get_length(&spec, 0.7 / 343.0 * 6.0) as usize; // 0.1 seconds samples delay
    let mut resonance_chamber = WaveGuide::new(delay_samples, 0.98, 1.0);

    let amplitude = std::i16::MAX as f32 * VOLUME;

    let mut filter = LowPassFilter::new(4000.0, onesec as u32);

    let rpmlo = 800.0f64;
    let rpmhi = 7000.0;

    for i in (0..get_length(&spec, duration)).map(|x| x as f64 / onesec) {
        let rpm = (i / 8.0).powf(0.9) * (rpmhi - rpmlo) + rpmlo;

        let x = (get_phasor_freq(rpm) * i) % 1.0;
        let mut sample = (exhaust_valve(x) * 1.0 + intake_valve(x) * 1.0 + piston_motion(x) * 0.5 + fuel_ignition(x) * 1.0) as f32;

        sample = filter.filter(sample);

        let (res, _) = resonance_chamber.step(sample, 0.0);
        sample += res;

        writer.write_sample((sample * amplitude) as i16).unwrap();
    }
}*/

struct WaveGuide {
    // goes from x0 to x1
    chamber0: DelayChamber,
    // goes from x1 to x0
    chamber1: DelayChamber,
    alpha:    f32,
    beta:     f32,
}

impl WaveGuide {
    pub fn new(delay: usize, alpha: f32, beta: f32) -> WaveGuide {
        WaveGuide {
            chamber0: DelayChamber::new(delay),
            chamber1: DelayChamber::new(delay),
            alpha,
            beta,
        }
    }

    pub fn step(&mut self, x0_in: f32, x1_in: f32) -> (f32, f32) {
        let c1_out = self.chamber1.pop();
        let c0_out = self.chamber0.pop();

        let ret = (c1_out * (1.0 - self.alpha.abs()), c0_out * (1.0 - self.beta.abs()));

        let c0_in = c1_out * self.alpha + x0_in;
        let c1_in = c0_out * self.beta + x1_in;

        self.chamber0.push(c0_in);
        self.chamber1.push(c1_in);
        self.chamber0.samples.advance();
        self.chamber1.samples.advance();

        ret
    }
}

#[derive(Clone)]
struct LoopBuffer<T> {
    len:      usize,
    pub data: Vec<T>,
    pos:      usize,
}

impl<T> LoopBuffer<T>
where T: Clone
{
    /// Creates a new loop buffer with specifies length.
    /// The internal sample buffer size is rounded up to the currently best SIMD implementation's float vector size.
    pub fn new(initial_value: T, len: usize) -> LoopBuffer<T> {
        simd_runtime_generate!(
            fn get_best_simd_size(size: usize) -> usize {
                ((size - 1) / S::VF32_WIDTH + 1) * S::VF32_WIDTH
            }
        );
        let bufsize = get_best_simd_size_runtime_select(len);
        LoopBuffer {
            len,
            data: std::iter::repeat(initial_value).take(bufsize).collect(),
            pos: 0,
        }
    }

    /// Sets the value at the current position. Must be called with `pop`.
    /// ```rust
    /// // assuming SIMD is in scalar mode
    /// let mut lb = LoopBuffer::new(2);
    /// lb.push(1.0);
    /// lb.advance();
    ///
    /// assert_eq(lb.pop(), 1.0);
    ///
    /// ```
    pub fn push(&mut self, value: T) {
        let len = self.len;
        self.data[self.pos % len] = value;
    }

    /// Gets the value `self.len` samples prior. Must be called with `push`.
    /// See `push` for examples
    pub fn pop(&mut self) -> T {
        let len = self.len;
        self.data[(self.pos + 1) % len].clone()
    }

    /// Advances the position of this loop buffer.
    pub fn advance(&mut self) {
        self.pos += 1;
    }
}

#[derive(Clone)]
struct LowPassFilter {
    samples:            LoopBuffer<f32>,
    samples_per_second: u32,
}

impl LowPassFilter {
    pub fn new(freq: f32, samples_per_second: u32) -> LowPassFilter {
        LowPassFilter {
            samples: LoopBuffer::new(0.0f32, ((samples_per_second as f32 / freq) as u32).min(samples_per_second).max(1) as usize),
            samples_per_second,
        }
    }

    pub fn filter(&mut self, sample: f32) -> f32 {
        self.samples.push(sample);
        self.samples.advance();

        unsafe {
            simd_runtime_generate!(
                fn sum(samples: &[f32]) -> f32 {
                    let mut i = S::VF32_WIDTH;
                    let len = samples.len();
                    assert_eq!(len % S::VF32_WIDTH, 0, "LoopBuffer length is not a multiple of the SIMD vector size");

                    // rolling sum
                    let mut sum = S::loadu_ps(&samples[0]);

                    while i != len {
                        sum += S::loadu_ps(&samples[i]);
                        i += S::VF32_WIDTH;
                    }
                    S::horizontal_add_ps(sum) / len as f32
                }
            );

            sum_runtime_select(&self.samples.data)
        }
    }
}

struct DelayChamber {
    samples: LoopBuffer<f32>,
}

impl DelayChamber {
    pub fn new(delay: usize) -> DelayChamber {
        DelayChamber {
            samples: LoopBuffer::new(0.0f32, delay)
        }
    }

    pub fn push(&mut self, sample: f32) {
        self.samples.push(sample);
    }

    pub fn pop(&mut self) -> f32 {
        self.samples.pop()
    }
}

fn get_length(spec: &AudioSpec, seconds: f64) -> u32 {
    (seconds * spec.channels as f64 * spec.freq as f64) as u32
}

fn get_phasor_freq(rpm: f64) -> f64 {
    rpm / 120.0
}

fn exhaust_valve(x: f64) -> f64 {
    if 0.75 < x && x < 1.0 {
        -(x * PI * 4.0).sin()
    } else {
        0.0
    }
}

fn intake_valve(x: f64) -> f64 {
    if 0.0 < x && x < 0.25 {
        (x * PI * 4.0).sin()
    } else {
        0.0
    }
}

fn piston_motion(x: f64) -> f64 {
    (x * PI * 4.0).cos()
}

fn fuel_ignition(x: f64) -> f64 {
    if 0.0 < x && x < IGNITION_SCALE {
        (2.0 * PI * (x * IGNITION_SCALE + 0.5)).sin()
    } else {
        0.0
    }
}
