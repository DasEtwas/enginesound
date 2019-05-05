//! ## Generator module ##
//!
//! Basic working principle:
//! Every sample-output generating object (Cylinder, WaveGuide, DelayLine, ..) has to be first `pop`ped,
//! it's output worked upon and then new input samples are `push`ed.
//!

use crate::{recorder::Recorder, SAMPLES_PER_CALLBACK};
use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use serde::{Deserialize, Serialize};
use simdeez::{avx2::*, scalar::*, sse2::*, sse41::*, *};
use std::{ops::{Deref, DerefMut},
          time::SystemTime};

pub const PI2F: f32 = 2.0 * std::f32::consts::PI;
pub const PI4F: f32 = 4.0 * std::f32::consts::PI;

// https://www.researchgate.net/profile/Stefano_Delle_Monache/publication/280086598_Physically_informed_car_engine_sound_synthesis_for_virtual_and_augmented_environments/links/55a791bc08aea2222c746724/Physically-informed-car-engine-sound-synthesis-for-virtual-and-augmented-environments.pdf?origin=publication_detail

#[derive(Serialize, Deserialize)]
pub struct Muffler {
    pub straight_pipe:    WaveGuide,
    pub muffler_elements: [WaveGuide; 4],
}

#[derive(Serialize, Deserialize)]
pub struct Engine {
    pub rpm: f32,

    pub cylinders: Vec<Cylinder>,
    #[serde(skip_serializing, skip_deserializing)]
    pub intake_noise: Noise,
    pub intake_noise_factor: f32,
    pub intake_lp_filter: LowPassFilter,
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
    pub crankshaft_pos: f32,
    pub exhaust_collector: f32,
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
    pub intake_open_refl:    f32,
    pub intake_closed_refl:  f32,
    pub exhaust_open_refl:   f32,
    pub exhaust_closed_refl: f32,

    pub piston_motion_factor: f32,
    pub ignition_factor: f32,
    /// the time it takes for the fuel to ignite in crank cycles (0.0 - 1.0)
    pub ignition_time: f32,
    /// time it takes for the pressure in the cylinder to release into exhaust/intake
    pub pressure_release_factor: f32,

    // running values
    pub cyl_sound:         f32,
    pub cyl_pressure:      f32,
    pub extractor_exhaust: f32,
}

impl Cylinder {
    /// takes in the current exhaust collector pressure
    /// returns intake, exhaust, piston + ignition values
    #[inline]
    pub(in crate::gen) fn pop(&mut self, crank_pos: f32, exhaust_collector: f32, intake_valve_shift: f32, exhaust_valve_shift: f32) -> (f32, f32, f32) {
        let crank = (crank_pos + self.crank_offset) % 1.0;

        self.cyl_sound = piston_motion(crank) * self.piston_motion_factor + fuel_ignition(crank, self.ignition_time) * self.ignition_factor;

        let ex_valve = exhaust_valve(crank + exhaust_valve_shift);
        let in_valve = intake_valve(crank + intake_valve_shift);

        self.exhaust_waveguide.alpha = (1.0 - ex_valve) * self.exhaust_open_refl + ex_valve * self.exhaust_closed_refl;
        self.intake_waveguide.alpha = (1.0 - in_valve) * self.intake_open_refl + in_valve * self.intake_closed_refl;

        // the first return value in the tuple is the cylinder-side valve-modulated side of the waveguide (alpha side)
        let ex_wg_ret = self.exhaust_waveguide.pop();
        let in_wg_ret = self.intake_waveguide.pop();

        let extractor_wg_ret = self.extractor_waveguide.pop();
        self.extractor_exhaust = extractor_wg_ret.0;
        self.extractor_waveguide.push(ex_wg_ret.1, exhaust_collector);

        self.cyl_pressure += self.cyl_sound + ex_wg_ret.0 + in_wg_ret.0;

        (in_wg_ret.1, extractor_wg_ret.1, self.cyl_sound)
    }

    /// called after pop
    pub(in crate::gen) fn push(&mut self, intake: f32) {
        let ex_in = self.exhaust_waveguide.alpha.abs() * self.cyl_pressure * 0.5 * self.pressure_release_factor;
        self.exhaust_waveguide.push(ex_in, self.extractor_exhaust);
        let in_in = self.intake_waveguide.alpha.abs() * self.cyl_pressure * 0.5 * self.pressure_release_factor;
        self.intake_waveguide.push(in_in, intake);

        self.cyl_pressure -= ex_in + in_in;
    }
}

#[derive(Serialize, Deserialize)]
pub struct Generator {
    pub sampler_duty: f32,
    #[serde(skip_serializing, skip_deserializing)]
    pub recorder: Option<Recorder>,
    #[serde(skip_serializing, skip_deserializing)]
    pub gui_graph: Vec<f32>,
    pub volume: f32,
    pub intake_volume: f32,
    pub exhaust_volume: f32,
    pub engine_vibrations_volume: f32,
    pub samples_per_second: u32,
    pub engine: Engine,
    /// `LowPassFilter` which is subtracted from the sample while playing back to reduce dc offset and thus clipping
    dc_lp: LowPassFilter,
}

impl Generator {
    pub(crate) fn new(samples_per_second: u32, engine: Engine, dc_lp: LowPassFilter) -> Generator {
        Generator {
            sampler_duty: 0.0_f32,
            recorder: None,
            gui_graph: vec![0.0; 2 * SAMPLES_PER_CALLBACK as usize],
            volume: 0.1_f32,
            intake_volume: 0.333_f32,
            exhaust_volume: 0.333_f32,
            engine_vibrations_volume: 0.333_f32,
            samples_per_second,
            engine,
            dc_lp,
        }
    }

    pub(crate) fn generate(&mut self, buf: &mut [f32]) {
        let crankshaft_pos = self.engine.crankshaft_pos;
        let samples_per_second = self.samples_per_second as f32 * 120.0;

        let mut i = 1.0;
        let mut ii = 0;
        while ii < buf.len() {
            self.engine.crankshaft_pos = (crankshaft_pos + i * self.get_rpm() / samples_per_second) % 1.0;
            let samples = self.gen();
            let sample = (samples.0 * self.get_intake_volume() + samples.1 * self.get_engine_vibrations_volume() + samples.2 * self.get_exhaust_volume())
                * self.get_volume();

            // reduces dc offset
            buf[ii] = sample - self.dc_lp.filter(sample);

            i += 1.0;
            ii += 1;
        }

        if let Some(recorder) = &mut self.recorder {
            recorder.record(buf.to_vec());
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
        }

        self.engine.muffler.straight_pipe.chamber0.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
        self.engine.muffler.straight_pipe.chamber1.samples.data.iter_mut().for_each(|sample| *sample = 0.0);
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
        self.intake_volume = intake_volume;
    }

    #[inline]
    pub fn get_intake_volume(&self) -> f32 {
        self.intake_volume
    }

    #[inline]
    pub fn set_exhaust_volume(&mut self, exhaust_volume: f32) {
        self.exhaust_volume = exhaust_volume;
    }

    #[inline]
    pub fn get_exhaust_volume(&self) -> f32 {
        self.exhaust_volume
    }

    #[inline]
    pub fn set_engine_vibrations_volume(&mut self, engine_vibrations_volume: f32) {
        self.engine_vibrations_volume = engine_vibrations_volume;
    }

    #[inline]
    pub fn get_engine_vibrations_volume(&self) -> f32 {
        self.engine_vibrations_volume
    }

    /// generates one sample worth of data
    /// returns  `(intake, engine vibrations, exhaust)`
    fn gen(&mut self) -> (f32, f32, f32) {
        let intake_noise = self.engine.intake_lp_filter.filter(self.engine.intake_noise.next_u32() as f32 / (std::u32::MAX as f32 / 2.0) - 1.0)
            * self.engine.intake_noise_factor;

        let mut engine_vibration = 0.0;

        let num_cyl = self.engine.cylinders.len() as f32;

        let last_exhaust_collector = self.engine.exhaust_collector / num_cyl;
        self.engine.exhaust_collector = 0.0;
        self.engine.intake_collector = 0.0;

        let crankshaft_fluctuation_offset =
            self.engine.crankshaft_fluctuation_lp.filter(self.engine.intake_noise.next_u32() as f32 / (std::u32::MAX as f32 / 2.0) - 1.0);

        for cylinder in self.engine.cylinders.iter_mut() {
            let (cyl_intake, cyl_exhaust, cyl_vib) = cylinder.pop(
                self.engine.crankshaft_pos + self.engine.crankshaft_fluctuation * crankshaft_fluctuation_offset,
                last_exhaust_collector,
                self.engine.intake_valve_shift,
                self.engine.exhaust_valve_shift,
            );
            self.engine.intake_collector += cyl_intake;
            self.engine.exhaust_collector += cyl_exhaust;
            engine_vibration += cyl_vib;
        }

        // parallel input to the exhaust straight pipe
        // alpha end is at exhaust collector
        let straight_pipe_wg_ret = self.engine.muffler.straight_pipe.pop();
        self.engine.exhaust_collector += straight_pipe_wg_ret.0;

        // alpha end is at straight pipe end (beta)
        let mut muffler_wg_ret = (0.0, 0.0);

        for muffler_line in self.engine.muffler.muffler_elements.iter_mut() {
            let ret = muffler_line.pop();
            muffler_wg_ret.0 += ret.0;
            muffler_wg_ret.1 += ret.1;
        }

        // pop  //
        //////////
        // push //

        let straight_pipe_out_muffler_in = (straight_pipe_wg_ret.1 + muffler_wg_ret.0) * 0.5;

        for cylinder in self.engine.cylinders.iter_mut() {
            // modulate intake
            cylinder.push(self.engine.intake_collector / num_cyl + intake_noise * intake_valve((self.engine.crankshaft_pos + cylinder.crank_offset) % 1.0));
        }

        self.engine.muffler.straight_pipe.push(self.engine.exhaust_collector, straight_pipe_out_muffler_in);

        let muffler_elements = self.engine.muffler.muffler_elements.len() as f32;

        for muffler_delay_line in self.engine.muffler.muffler_elements.iter_mut() {
            muffler_delay_line.push(straight_pipe_out_muffler_in / muffler_elements, 0.0);
        }

        engine_vibration = self.engine.engine_vibration_filter.filter(engine_vibration);

        (self.engine.intake_collector, engine_vibration, muffler_wg_ret.1)
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
    c1_out: f32,
    c0_out: f32,
}

impl WaveGuide {
    pub fn new(delay: usize, alpha: f32, beta: f32) -> WaveGuide {
        WaveGuide {
            chamber0: DelayLine::new(delay),
            chamber1: DelayLine::new(delay),
            alpha,
            beta,
            c1_out: 0.0,
            c0_out: 0.0,
        }
    }

    pub fn pop(&mut self) -> (f32, f32) {
        self.c1_out = self.chamber1.pop();
        self.c0_out = self.chamber0.pop();

        let ret = (self.c1_out * (1.0 - self.alpha.abs()), self.c0_out * (1.0 - self.beta.abs()));

        ret
    }

    pub fn push(&mut self, x0_in: f32, x1_in: f32) {
        let c0_in = self.c1_out * self.alpha + x0_in;
        let c1_in = self.c0_out * self.beta + x1_in;

        self.chamber0.push(c0_in);
        self.chamber1.push(c1_in);
        self.chamber0.samples.advance();
        self.chamber1.samples.advance();
    }

    pub fn update(&mut self, delay: usize, alpha: f32, beta: f32) -> Option<Self> {
        if delay != self.chamber0.samples.len || alpha != self.alpha || beta != self.beta {
            Some(Self::new(delay, alpha, beta))
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoopBuffer<T> {
    pub len: usize,
    #[serde(skip_serializing, skip_deserializing)]
    pub(in crate::gen) data: Vec<T>,
    pos: usize,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct LowPassFilter {
    pub samples:            LoopBuffer<f32>,
    pub samples_per_second: u32,
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

    pub fn update(&mut self, freq: f32) -> Option<Self> {
        let newfreq_samples = ((self.samples_per_second as f32 / freq) as u32).min(self.samples_per_second).max(1) as usize;
        if newfreq_samples != self.samples.len {
            Some(Self::new(freq, self.samples_per_second))
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DelayLine {
    pub samples: LoopBuffer<f32>,
}

impl DelayLine {
    pub fn new(delay: usize) -> DelayLine {
        DelayLine {
            samples: LoopBuffer::new(0.0f32, delay)
        }
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
