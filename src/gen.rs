//! ## Generator module ##
//!
//! Basic working principle:
//! Every sample-output generating object (Cylinder, WaveGuide, DelayLine, ..) has to be first `pop`ped,
//! it's output worked upon and then new input samples are `push`ed.
//!

use crate::{recorder::Recorder, SAMPLES_PER_CALLBACK};

#[allow(unused_imports)]
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[allow(unused_imports)]
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use serde::{Deserialize, Serialize};
use simdeez::{avx2::*, scalar::*, sse2::*, sse41::*, *};
use std::{
    ops::{Deref, DerefMut},
    time::SystemTime,
};

pub const PI2F: f32 = 2.0 * std::f32::consts::PI;
pub const PI4F: f32 = 4.0 * std::f32::consts::PI;
pub const WAVEGUIDE_MAX_AMP: f32 = 20.0; // at this amplitude, a reciprocal damping function is applied to fight feedback loops

// https://www.researchgate.net/profile/Stefano_Delle_Monache/publication/280086598_Physically_informed_car_engine_sound_synthesis_for_virtual_and_augmented_environments/links/55a791bc08aea2222c746724/Physically-informed-car-engine-sound-synthesis-for-virtual-and-augmented-environments.pdf?origin=publication_detail

#[derive(Serialize, Deserialize)]
pub struct Muffler {
    pub straight_pipe: WaveGuide,
    pub muffler_elements: Vec<WaveGuide>,
}

#[derive(Serialize, Deserialize)]
pub struct Engine {
    pub rpm: f32,
    pub intake_volume: f32,
    pub exhaust_volume: f32,
    pub engine_vibrations_volume: f32,

    pub cylinders: Vec<Cylinder>,
    #[serde(skip)]
    pub intake_noise: Noise,
    pub intake_noise_factor: f32,
    pub intake_noise_lp: LowPassFilter,
    pub engine_vibration_filter: LowPassFilter,
    pub muffler: Muffler,
    /// valve timing -0.5 - 0.5
    pub intake_valve_shift: f32,
    /// valve timing -0.5 - 0.5
    pub exhaust_valve_shift: f32,
    pub crankshaft_fluctuation: f32,
    pub crankshaft_fluctuation_lp: LowPassFilter,
    // running values
    /// crankshaft position, 0.0-1.0
    #[serde(skip)]
    pub crankshaft_pos: f32,
    #[serde(skip)]
    pub exhaust_collector: f32,
    #[serde(skip)]
    pub intake_collector: f32,
}

pub struct Noise {
    inner: XorShiftRng,
}

impl Default for Noise {
    fn default() -> Self {
        Noise {
            inner: XorShiftRng::from_seed(unsafe {
                std::mem::transmute::<u128, [u8; 16]>(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos())
            }),
        }
    }
}

impl Deref for Noise {
    type Target = XorShiftRng;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Noise {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Represents one audio cylinder
/// It has two `WaveGuide`s each connected from the cylinder to the exhaust or intake collector
/// ```
/// Labels:                                                     \/ Extractor
///                    b      a            a      b           a    b
/// (Intake Collector) <==|IV|> (Cylinder) <|EV|==> (Exhaust) <====> (Exhaust collector)
///
/// a   b
/// <===>   - WaveGuide with alpha / beta sides => alpha controls the reflectiveness of that side
///
/// |IV|    - Intake valve modulation function for this side of the WaveGuide (alpha)
///
/// |EV|    - Exhaust valve modulation function for this side of the WaveGuide (alpha)
/// ```
#[derive(Serialize, Deserialize, Clone)]
pub struct Cylinder {
    /// offset of this cylinder's piston crank
    pub crank_offset: f32,
    /// waveguide from the cylinder to the exhaust
    pub exhaust_waveguide: WaveGuide,
    /// waveguide from the cylinder to the intake
    pub intake_waveguide: WaveGuide,
    /// waveguide from the other end of the exhaust WG to the exhaust collector
    pub extractor_waveguide: WaveGuide,
    // waveguide alpha values for when the valves are closed or opened
    pub intake_open_refl: f32,
    pub intake_closed_refl: f32,
    pub exhaust_open_refl: f32,
    pub exhaust_closed_refl: f32,

    pub piston_motion_factor: f32,
    pub ignition_factor: f32,
    /// the time it takes for the fuel to ignite in crank cycles (0.0 - 1.0)
    pub ignition_time: f32,

    // running values
    #[serde(skip)]
    pub cyl_sound: f32,
    #[serde(skip)]
    pub extractor_exhaust: f32,
}

impl Cylinder {
    /// takes in the current exhaust collector pressure
    /// returns (intake, exhaust, piston + ignition, waveguide dampened)
    #[inline]
    pub(in crate::gen) fn pop(&mut self, crank_pos: f32, exhaust_collector: f32, intake_valve_shift: f32, exhaust_valve_shift: f32) -> (f32, f32, f32, bool) {
        let crank = (crank_pos + self.crank_offset) % 1.0;

        self.cyl_sound = piston_motion(crank) * self.piston_motion_factor + fuel_ignition(crank, self.ignition_time) * self.ignition_factor;

        let ex_valve = exhaust_valve(crank + exhaust_valve_shift);
        let in_valve = intake_valve(crank + intake_valve_shift);

        self.exhaust_waveguide.alpha = self.exhaust_closed_refl + (self.exhaust_open_refl - self.exhaust_closed_refl) * ex_valve;
        self.intake_waveguide.alpha = self.intake_closed_refl + (self.intake_open_refl - self.intake_closed_refl) * in_valve;

        // the first return value in the tuple is the cylinder-side valve-modulated side of the waveguide (alpha side)
        let ex_wg_ret = self.exhaust_waveguide.pop();
        let in_wg_ret = self.intake_waveguide.pop();

        let extractor_wg_ret = self.extractor_waveguide.pop();
        self.extractor_exhaust = extractor_wg_ret.0;
        self.extractor_waveguide.push(ex_wg_ret.1, exhaust_collector);

        //self.cyl_sound += ex_wg_ret.0 + in_wg_ret.0;

        (in_wg_ret.1, extractor_wg_ret.1, self.cyl_sound, ex_wg_ret.2 | in_wg_ret.2 | extractor_wg_ret.2)
    }

    /// called after pop
    pub(in crate::gen) fn push(&mut self, intake: f32) {
        let ex_in = (1.0 - self.exhaust_waveguide.alpha.abs()) * self.cyl_sound * 0.5;
        self.exhaust_waveguide.push(ex_in, self.extractor_exhaust);
        let in_in = (1.0 - self.intake_waveguide.alpha.abs()) * self.cyl_sound * 0.5;
        self.intake_waveguide.push(in_in, intake);
    }
}

pub struct Generator {
    pub sampler_duty: f32,
    pub recorder: Option<Recorder>,
    pub gui_graph: Vec<f32>,
    pub volume: f32,
    pub samples_per_second: u32,
    pub engine: Engine,
    /// `LowPassFilter` which is subtracted from the sample while playing back to reduce dc offset and thus clipping
    dc_lp: LowPassFilter,
    /// set to true by any waveguide if it is dampening it's output to prevent feedback loops
    pub waveguides_dampened: bool,
    /// set to true if the amplitude of the recording is greater than 1
    pub recording_currently_clipping: bool,
}

impl Generator {
    pub(crate) fn new(samples_per_second: u32, engine: Engine, dc_lp: LowPassFilter) -> Generator {
        Generator {
            sampler_duty: 0.0_f32,
            recorder: None,
            gui_graph: vec![0.0; 2 * SAMPLES_PER_CALLBACK as usize],
            volume: 0.1_f32,
            samples_per_second,
            engine,
            dc_lp,
            waveguides_dampened: false,
            recording_currently_clipping: false,
        }
    }

    pub(crate) fn generate(&mut self, buf: &mut [f32]) {
        let crankshaft_pos = self.engine.crankshaft_pos;
        let samples_per_second = self.samples_per_second as f32 * 120.0;

        self.recording_currently_clipping = false;
        self.waveguides_dampened = false;

        let mut i = 1.0;
        let mut ii = 0;
        while ii < buf.len() {
            self.engine.crankshaft_pos = (crankshaft_pos + i * self.get_rpm() / samples_per_second) % 1.0;
            let samples = self.gen();
            let sample = (samples.0 * self.get_intake_volume() + samples.1 * self.get_engine_vibrations_volume() + samples.2 * self.get_exhaust_volume())
                * self.get_volume();
            self.waveguides_dampened |= samples.3;

            // reduces dc offset
            buf[ii] = sample - self.dc_lp.filter(sample);

            i += 1.0;
            ii += 1;
        }

        if let Some(recorder) = &mut self.recorder {
            let bufvec = buf.to_vec();
            let mut recording_currently_clipping = false;
            bufvec.iter().for_each(|sample| recording_currently_clipping |= sample.abs() > 1.0);
            self.recording_currently_clipping = recording_currently_clipping;

            recorder.record(bufvec);
        }

        self.gui_graph.clear();
        self.gui_graph.extend_from_slice(buf);
    }

    pub fn reset(&mut self) {
        for cyl in self.engine.cylinders.iter_mut() {
            cyl.exhaust_waveguide.chamber0.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
            cyl.exhaust_waveguide.chamber1.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
            cyl.intake_waveguide.chamber0.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
            cyl.intake_waveguide.chamber1.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
            cyl.extractor_waveguide.chamber0.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
            cyl.extractor_waveguide.chamber1.samples.data.iter_mut().for_each(|sample| *sample = 0.0);

            cyl.extractor_exhaust = 0.0;
            cyl.cyl_sound = 0.0;
        }

        self.engine.muffler.straight_pipe.chamber0.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
        self.engine.muffler.straight_pipe.chamber1.samples.data.iter_mut().for_each(|sample| *sample = 0.0);

        self.engine.engine_vibration_filter.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
        self.engine.engine_vibration_filter.samples.data.iter_mut().for_each(|sample| *sample = 0.0);

        self.engine.crankshaft_fluctuation_lp.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
        self.engine.crankshaft_fluctuation_lp.samples.data.iter_mut().for_each(|sample| *sample = 0.0);

        for muffler_element in self.engine.muffler.muffler_elements.iter_mut() {
            muffler_element.chamber0.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
            muffler_element.chamber1.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
        }

        self.engine.exhaust_collector = 0.0;
        self.engine.intake_collector = 0.0;
    }

    #[inline]
    pub fn get_rpm(&self) -> f32 {
        self.engine.rpm
    }

    #[inline]
    pub fn set_rpm(&mut self, rpm: f32) {
        self.engine.rpm = rpm;
    }

    #[inline]
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }

    #[inline]
    pub fn get_volume(&self) -> f32 {
        self.volume
    }

    #[inline]
    pub fn set_intake_volume(&mut self, intake_volume: f32) {
        self.engine.intake_volume = intake_volume;
    }

    #[inline]
    pub fn get_intake_volume(&self) -> f32 {
        self.engine.intake_volume
    }

    #[inline]
    pub fn set_exhaust_volume(&mut self, exhaust_volume: f32) {
        self.engine.exhaust_volume = exhaust_volume;
    }

    #[inline]
    pub fn get_exhaust_volume(&self) -> f32 {
        self.engine.exhaust_volume
    }

    #[inline]
    pub fn set_engine_vibrations_volume(&mut self, engine_vibrations_volume: f32) {
        self.engine.engine_vibrations_volume = engine_vibrations_volume;
    }

    #[inline]
    pub fn get_engine_vibrations_volume(&self) -> f32 {
        self.engine.engine_vibrations_volume
    }

    /// generates one sample worth of data
    /// returns  `(intake, engine vibrations, exhaust, waveguides dampened)`
    fn gen(&mut self) -> (f32, f32, f32, bool) {
        let intake_noise = self.engine.intake_noise_lp.filter(self.engine.intake_noise.next_u32() as f32 / (std::u32::MAX as f32 / 2.0) - 1.0)
            * self.engine.intake_noise_factor;

        let mut engine_vibration = 0.0;

        let num_cyl = self.engine.cylinders.len() as f32;

        let last_exhaust_collector = self.engine.exhaust_collector / num_cyl;
        self.engine.exhaust_collector = 0.0;
        self.engine.intake_collector = 0.0;

        let crankshaft_fluctuation_offset =
            self.engine.crankshaft_fluctuation_lp.filter(self.engine.intake_noise.next_u32() as f32 / (std::u32::MAX as f32 / 2.0) - 1.0);

        let mut cylinder_dampened = false;

        for cylinder in self.engine.cylinders.iter_mut() {
            let (cyl_intake, cyl_exhaust, cyl_vib, dampened) = cylinder.pop(
                self.engine.crankshaft_pos + self.engine.crankshaft_fluctuation * crankshaft_fluctuation_offset,
                last_exhaust_collector,
                self.engine.intake_valve_shift,
                self.engine.exhaust_valve_shift,
            );
            self.engine.intake_collector += cyl_intake;
            self.engine.exhaust_collector += cyl_exhaust;
            engine_vibration += cyl_vib;
            cylinder_dampened |= dampened;
        }

        // parallel input to the exhaust straight pipe
        // alpha end is at exhaust collector
        let straight_pipe_wg_ret = self.engine.muffler.straight_pipe.pop();

        // alpha end is at straight pipe end (beta)
        let mut muffler_wg_ret = (0.0, 0.0, false);

        for muffler_line in self.engine.muffler.muffler_elements.iter_mut() {
            let ret = muffler_line.pop();
            muffler_wg_ret.0 += ret.0;
            muffler_wg_ret.1 += ret.1;
            muffler_wg_ret.2 |= ret.2;
        }

        // pop  //
        //////////
        // push //

        for cylinder in self.engine.cylinders.iter_mut() {
            // modulate intake
            cylinder.push(self.engine.intake_collector / num_cyl + intake_noise * intake_valve((self.engine.crankshaft_pos + cylinder.crank_offset) % 1.0));
        }

        self.engine.muffler.straight_pipe.push(self.engine.exhaust_collector, muffler_wg_ret.0);

        self.engine.exhaust_collector += straight_pipe_wg_ret.0;

        let muffler_elements = self.engine.muffler.muffler_elements.len() as f32;

        for muffler_delay_line in self.engine.muffler.muffler_elements.iter_mut() {
            muffler_delay_line.push(straight_pipe_wg_ret.1 / muffler_elements, 0.0);
        }

        engine_vibration = self.engine.engine_vibration_filter.filter(engine_vibration);

        (self.engine.intake_collector, engine_vibration, muffler_wg_ret.1, straight_pipe_wg_ret.2 | cylinder_dampened)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WaveGuide {
    // goes from x0 to x1
    pub chamber0: DelayLine,
    // goes from x1 to x0
    chamber1: DelayLine,
    /// reflection factor for the first value of the return tuple of `pop`
    pub alpha: f32,
    /// reflection factor for the second value of the return tuple of `pop`
    pub beta: f32,

    // running values
    #[serde(skip)]
    c1_out: f32,
    #[serde(skip)]
    c0_out: f32,
}

impl WaveGuide {
    pub fn new(delay: usize, alpha: f32, beta: f32, samples_per_second: u32) -> WaveGuide {
        WaveGuide {
            chamber0: DelayLine::new(delay, samples_per_second),
            chamber1: DelayLine::new(delay, samples_per_second),
            alpha,
            beta,
            c1_out: 0.0,
            c0_out: 0.0,
        }
    }

    pub fn pop(&mut self) -> (f32, f32, bool) {
        let (c1_out, dampened_c1) = WaveGuide::dampen(self.chamber1.pop());;
        let (c0_out, dampened_c0) = WaveGuide::dampen(self.chamber0.pop());;
        self.c1_out = c1_out;
        self.c0_out = c0_out;

        (self.c1_out * (1.0 - self.alpha.abs()), self.c0_out * (1.0 - self.beta.abs()), dampened_c1 | dampened_c0)
    }
    #[inline]
    pub fn dampen(sample: f32) -> (f32, bool) {
        let sample_abs = sample.abs();
        if sample_abs > WAVEGUIDE_MAX_AMP {
            (sample.signum() * (-1.0 / (sample_abs - WAVEGUIDE_MAX_AMP + 1.0) + 1.0 + WAVEGUIDE_MAX_AMP), true)
        } else {
            (sample, false)
        }
    }

    pub fn push(&mut self, x0_in: f32, x1_in: f32) {
        let c0_in = self.c1_out * self.alpha + x0_in;
        let c1_in = self.c0_out * self.beta + x1_in;

        self.chamber0.push(c0_in);
        self.chamber1.push(c1_in);
        self.chamber0.samples.advance();
        self.chamber1.samples.advance();
    }

    pub fn update(&mut self, delay: usize, alpha: f32, beta: f32, samples_per_second: u32) -> Option<Self> {
        if delay != self.chamber0.samples.len || alpha != self.alpha || beta != self.beta {
            Some(Self::new(delay, alpha, beta, samples_per_second))
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(from = "crate::deser::LoopBufferDeser")]
pub struct LoopBuffer {
    // in seconds
    pub delay: f32,
    #[serde(skip)]
    pub len: usize,
    #[serde(skip)]
    pub data: Vec<f32>,
    #[serde(skip)]
    pub pos: usize,
}

impl LoopBuffer {
    /// Creates a new loop buffer with specifies length.
    /// The internal sample buffer size is rounded up to the currently best SIMD implementation's float vector size.
    pub fn new(len: usize, samples_per_second: u32) -> LoopBuffer {
        let bufsize = LoopBuffer::get_best_simd_size(len);
        LoopBuffer { delay: len as f32 / samples_per_second as f32, len, data: vec![0.0; bufsize], pos: 0 }
    }

    /// Returns `(size / SIMD_REGISTER_SIZE).ceil() * SIMD_REGISTER_SIZE`, where `SIMD` may be the best simd implementation at runtime.
    /// Used to create vectors to make simd iteration easier
    pub fn get_best_simd_size(size: usize) -> usize {
        if is_x86_feature_detected!("avx2") {
            ((size - 1) / Avx2::VF32_WIDTH + 1) * Avx2::VF32_WIDTH
        } else if is_x86_feature_detected!("sse4.1") {
            ((size - 1) / Sse41::VF32_WIDTH + 1) * Sse41::VF32_WIDTH
        } else if is_x86_feature_detected!("sse2") {
            ((size - 1) / Sse2::VF32_WIDTH + 1) * Sse2::VF32_WIDTH
        } else {
            ((size - 1) / Scalar::VF32_WIDTH + 1) * Scalar::VF32_WIDTH
        }
    }

    /// Sets the value at the current position. Must be called with `pop`.
    /// ```rust
    /// // assuming Simd is Scalar
    /// let mut lb = LoopBuffer::new(2);
    /// lb.push(1.0);
    /// lb.advance();
    ///
    /// assert_eq(lb.pop(), 1.0);
    ///
    /// ```
    pub fn push(&mut self, value: f32) {
        let len = self.len;
        self.data[self.pos % len] = value;
    }

    /// Gets the value `self.len` samples prior. Must be called with `push`.
    /// See `push` for examples
    pub fn pop(&mut self) -> f32 {
        let len = self.len;
        self.data[(self.pos + 1) % len].clone()
    }

    /// Advances the position of this loop buffer.
    pub fn advance(&mut self) {
        self.pos += 1;
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LowPassFilter {
    pub samples: LoopBuffer,
    pub len: f32,
}

impl LowPassFilter {
    pub fn new(freq: f32, samples_per_second: u32) -> LowPassFilter {
        let len = (samples_per_second as f32 / freq).min(samples_per_second as f32).max(1.0);
        LowPassFilter { samples: LoopBuffer::new(len.ceil() as usize, samples_per_second), len }
    }

    #[inline]
    pub fn get_freq(&self, samples_per_second: u32) -> f32 {
        samples_per_second as f32 / self.len
    }

    pub fn filter(&mut self, sample: f32) -> f32 {
        self.samples.push(sample);
        self.samples.advance();

        #[inline(always)]
        unsafe fn sum<S: Simd>(samples: &[f32], flen: f32) -> f32 {
            let mut i = S::VF32_WIDTH;
            let len = samples.len();
            assert_eq!(len % S::VF32_WIDTH, 0, "LoopBuffer length is not a multiple of the SIMD vector size");

            // rolling sum
            let mut rolling_sum = S::loadu_ps(&samples[0]);

            while i != len {
                rolling_sum += S::loadu_ps(&samples[i]);
                i += S::VF32_WIDTH;
            }

            let fract = flen.fract();
            // only use fractional averaging if flen.fract() > 0.0
            if fract != 0.0 {
                // subtract the last element and add it onto the sum again but multiplied with the fractional part of the length
                (S::horizontal_add_ps(rolling_sum) - samples[flen as usize] * (1.0 - fract)) / flen
            } else {
                // normal average
                S::horizontal_add_ps(rolling_sum) / flen
            }
        }

        // expanded 'simd_runtime_select' macro for feature independency (proc_macro_hygiene)
        if is_x86_feature_detected!("avx2") {
            #[target_feature(enable = "avx2")]
            unsafe fn call(samples: &[f32], len: f32) -> f32 {
                sum::<Avx2>(samples, len)
            }
            unsafe { call(&self.samples.data, self.len) }
        } else if is_x86_feature_detected!("sse4.1") {
            #[target_feature(enable = "sse4.1")]
            unsafe fn call(samples: &[f32], len: f32) -> f32 {
                sum::<Sse41>(samples, len)
            }
            unsafe { call(&self.samples.data, self.len) }
        } else if is_x86_feature_detected!("sse2") {
            #[target_feature(enable = "sse2")]
            unsafe fn call(samples: &[f32], len: f32) -> f32 {
                sum::<Sse2>(samples, len)
            }
            unsafe { call(&self.samples.data, self.len) }
        } else {
            unsafe { sum::<Scalar>(&self.samples.data, self.len) }
        }
    }

    pub fn update(&mut self, freq: f32, samples_per_second: u32) -> Option<Self> {
        let newfreq_len = (samples_per_second as f32 / freq).min(samples_per_second as f32).max(1.0);

        if newfreq_len != self.len {
            Some(Self::new(freq, samples_per_second))
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DelayLine {
    pub samples: LoopBuffer,
}

impl DelayLine {
    pub fn new(delay: usize, samples_per_second: u32) -> DelayLine {
        DelayLine { samples: LoopBuffer::new(delay, samples_per_second) }
    }

    pub fn pop(&mut self) -> f32 {
        self.samples.pop()
    }

    pub fn push(&mut self, sample: f32) {
        self.samples.push(sample);
    }
}

fn exhaust_valve(crank_pos: f32) -> f32 {
    if 0.75 < crank_pos && crank_pos < 1.0 {
        -(crank_pos * PI4F).sin()
    } else {
        0.0
    }
}

fn intake_valve(crank_pos: f32) -> f32 {
    if 0.0 < crank_pos && crank_pos < 0.25 {
        (crank_pos * PI4F).sin()
    } else {
        0.0
    }
}

fn piston_motion(crank_pos: f32) -> f32 {
    (crank_pos * PI4F).cos()
}

fn fuel_ignition(crank_pos: f32, ignition_time: f32) -> f32 {
    if 0.0 < crank_pos && crank_pos < ignition_time {
        (PI2F * (crank_pos * ignition_time + 0.5)).sin()
    } else {
        0.0
    }
}
