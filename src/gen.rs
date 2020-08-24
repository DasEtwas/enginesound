//! ## Generator module ##
//!
//! Basic working principle:
//! Every sample-output generating object (Cylinder, WaveGuide, DelayLine, ..) has to be first `pop`ped,
//! it's output worked upon and then new input samples are `push`ed.
//!

use crate::recorder::Recorder;

use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub const PI2F: f32 = 2.0 * std::f32::consts::PI;
pub const PI4F: f32 = 4.0 * std::f32::consts::PI;
pub const WAVEGUIDE_MAX_AMP: f32 = 20.0; // at this amplitude, a damping function is applied to fight feedback loops

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
    #[serde(skip)]
    pub crankshaft_noise: Noise,
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
                std::mem::transmute::<u128, [u8; 16]>(
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                )
            }),
        }
    }
}

impl Noise {
    pub fn step(&mut self) -> f32 {
        self.inner.next_u32() as f32 / (std::u32::MAX as f32 / 2.0) - 1.0
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
    pub(in crate::gen) fn pop(
        &mut self,
        crank_pos: f32,
        exhaust_collector: f32,
        intake_valve_shift: f32,
        exhaust_valve_shift: f32,
    ) -> (f32, f32, f32, bool) {
        let crank = (crank_pos + self.crank_offset).fract();

        self.cyl_sound = piston_motion(crank) * self.piston_motion_factor
            + fuel_ignition(crank, self.ignition_time) * self.ignition_factor;

        let ex_valve = exhaust_valve((crank + exhaust_valve_shift).fract());
        let in_valve = intake_valve((crank + intake_valve_shift).fract());

        self.exhaust_waveguide.alpha = self.exhaust_closed_refl
            + (self.exhaust_open_refl - self.exhaust_closed_refl) * ex_valve;
        self.intake_waveguide.alpha =
            self.intake_closed_refl + (self.intake_open_refl - self.intake_closed_refl) * in_valve;

        // the first return value in the tuple is the cylinder-side valve-modulated side of the waveguide (alpha side)
        let ex_wg_ret = self.exhaust_waveguide.pop();
        let in_wg_ret = self.intake_waveguide.pop();

        let extractor_wg_ret = self.extractor_waveguide.pop();
        self.extractor_exhaust = extractor_wg_ret.0;
        self.extractor_waveguide
            .push(ex_wg_ret.1, exhaust_collector);

        //self.cyl_sound += ex_wg_ret.0 + in_wg_ret.0;

        (
            in_wg_ret.1,
            extractor_wg_ret.1,
            self.cyl_sound,
            ex_wg_ret.2 | in_wg_ret.2 | extractor_wg_ret.2,
        )
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
    pub(crate) recorder: Option<Recorder>,
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
    pub fn new(samples_per_second: u32, engine: Engine, dc_lp: LowPassFilter) -> Generator {
        Generator {
            recorder: None,
            volume: 0.1_f32,
            samples_per_second,
            engine,
            dc_lp,
            waveguides_dampened: false,
            recording_currently_clipping: false,
        }
    }

    pub fn generate(&mut self, buf: &mut [f32]) {
        let samples_per_second = self.samples_per_second as f32 * 120.0;

        self.recording_currently_clipping = false;
        self.waveguides_dampened = false;

        let inc = self.engine.rpm / samples_per_second;

        buf.iter_mut().for_each(|sample| {
            self.engine.crankshaft_pos = (self.engine.crankshaft_pos + inc).fract();

            let channels = self.gen();
            let mixed = (channels.0 * self.engine.intake_volume
                + channels.1 * self.engine.engine_vibrations_volume
                + channels.2 * self.engine.exhaust_volume)
                * self.volume;
            self.waveguides_dampened |= channels.3;

            // reduces dc offset
            *sample = mixed - self.dc_lp.filter(mixed);
        });

        if let Some(recorder) = &mut self.recorder {
            let bufvec = buf.to_vec();
            let mut recording_currently_clipping = false;
            bufvec
                .iter()
                .for_each(|sample| recording_currently_clipping |= sample.abs() > 1.0);
            self.recording_currently_clipping = recording_currently_clipping;

            recorder.record(bufvec);
        }
    }

    pub fn reset(&mut self) {
        for cyl in self.engine.cylinders.iter_mut() {
            [
                &mut cyl.exhaust_waveguide,
                &mut cyl.intake_waveguide,
                &mut cyl.extractor_waveguide,
            ]
            .iter_mut()
            .flat_map(|x| vec![&mut x.chamber0, &mut x.chamber1])
            .for_each(|chamber| chamber.samples.data.iter_mut().for_each(|x| *x = 0.0));

            cyl.extractor_exhaust = 0.0;
            cyl.cyl_sound = 0.0;
        }

        std::iter::once(&mut self.engine.muffler.straight_pipe)
            .flat_map(|x| vec![&mut x.chamber0, &mut x.chamber1])
            .for_each(|chamber| chamber.samples.data.iter_mut().for_each(|x| *x = 0.0));

        for muffler_element in self.engine.muffler.muffler_elements.iter_mut() {
            muffler_element
                .chamber0
                .samples
                .data
                .iter_mut()
                .for_each(|sample| *sample = 0.0);
            muffler_element
                .chamber1
                .samples
                .data
                .iter_mut()
                .for_each(|sample| *sample = 0.0);
        }

        self.engine.exhaust_collector = 0.0;
        self.engine.intake_collector = 0.0;
    }

    /// generates one sample worth of audio
    /// returns  `(intake, engine vibrations, exhaust, waveguides dampened)`
    fn gen(&mut self) -> (f32, f32, f32, bool) {
        let intake_noise = self
            .engine
            .intake_noise_lp
            .filter(self.engine.intake_noise.step())
            * self.engine.intake_noise_factor;

        let mut engine_vibration = 0.0;

        let num_cyl = self.engine.cylinders.len() as f32;

        let last_exhaust_collector = self.engine.exhaust_collector / num_cyl;
        self.engine.exhaust_collector = 0.0;
        self.engine.intake_collector = 0.0;

        let crankshaft_fluctuation_offset = self
            .engine
            .crankshaft_fluctuation_lp
            .filter(self.engine.crankshaft_noise.step());

        let mut cylinder_dampened = false;

        for cylinder in self.engine.cylinders.iter_mut() {
            let (cyl_intake, cyl_exhaust, cyl_vib, dampened) = cylinder.pop(
                self.engine.crankshaft_pos
                    + self.engine.crankshaft_fluctuation * crankshaft_fluctuation_offset,
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
            cylinder.push(
                self.engine.intake_collector / num_cyl
                    + intake_noise
                        * intake_valve(
                            (self.engine.crankshaft_pos + cylinder.crank_offset).fract(),
                        ),
            );
        }

        self.engine
            .muffler
            .straight_pipe
            .push(self.engine.exhaust_collector, muffler_wg_ret.0);

        self.engine.exhaust_collector += straight_pipe_wg_ret.0;

        let muffler_elements = self.engine.muffler.muffler_elements.len() as f32;

        for muffler_delay_line in self.engine.muffler.muffler_elements.iter_mut() {
            muffler_delay_line.push(straight_pipe_wg_ret.1 / muffler_elements, 0.0);
        }

        engine_vibration = self.engine.engine_vibration_filter.filter(engine_vibration);

        (
            self.engine.intake_collector,
            engine_vibration,
            muffler_wg_ret.1,
            straight_pipe_wg_ret.2 | cylinder_dampened,
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WaveGuide {
    // goes from x0 to x1
    pub chamber0: DelayLine,
    // goes from x1 to x0
    pub chamber1: DelayLine,
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
        let (c1_out, dampened_c1) = WaveGuide::dampen(self.chamber1.pop());
        let (c0_out, dampened_c0) = WaveGuide::dampen(self.chamber0.pop());
        self.c1_out = c1_out;
        self.c0_out = c0_out;

        (
            self.c1_out * (1.0 - self.alpha.abs()),
            self.c0_out * (1.0 - self.beta.abs()),
            dampened_c1 | dampened_c0,
        )
    }
    #[inline]
    pub fn dampen(sample: f32) -> (f32, bool) {
        let sample_abs = sample.abs();
        if sample_abs > WAVEGUIDE_MAX_AMP {
            (
                sample.signum()
                    * (-1.0 / (sample_abs - WAVEGUIDE_MAX_AMP + 1.0) + 1.0 + WAVEGUIDE_MAX_AMP),
                true,
            )
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

    #[allow(clippy::float_cmp)]
    pub fn get_changed(
        &mut self,
        delay: usize,
        alpha: f32,
        beta: f32,
        samples_per_second: u32,
    ) -> Option<Self> {
        // the strictly compared values will never change without user interaction (adjusting sliders)
        if delay != self.chamber0.samples.data.len() || alpha != self.alpha || beta != self.beta {
            let mut new = Self::new(delay, alpha, beta, samples_per_second);

            // used to reduce artifacts while resizing pipes _a bit_
            fn copy_samples_faded(source: &[f32], dest: &mut [f32]) {
                let min_len = source.len().min(dest.len());

                dest[0..min_len].copy_from_slice(&source[0..min_len]);
                let (a, b) = (*source.last().unwrap(), source[0]);
                let dest_len = dest.len();
                dest[min_len..]
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, x)| *x = a + (b - a) * i as f32 / (dest_len - min_len) as f32);
            }

            copy_samples_faded(&self.chamber0.samples.data, &mut new.chamber0.samples.data);
            copy_samples_faded(&self.chamber1.samples.data, &mut new.chamber1.samples.data);

            Some(new)
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LoopBuffer {
    // in seconds
    pub delay: f32,
    #[serde(skip)]
    pub data: Vec<f32>,
    #[serde(skip)]
    pub pos: usize,
}

impl LoopBuffer {
    /// Creates a new loop buffer with specifies length.
    /// The internal sample buffer size is rounded up to the currently best SIMD implementation's float vector size.
    pub fn new(len: usize, samples_per_second: u32) -> LoopBuffer {
        LoopBuffer {
            delay: len as f32 / samples_per_second as f32,
            data: vec![0.0; len],
            pos: 0,
        }
    }

    /// Sets the value at the current position. Must be called with `pop`.
    /// ```rust
    /// let mut lb = LoopBuffer::new(2);
    /// lb.push(1.0);
    /// lb.advance();
    ///
    /// assert_eq(lb.pop(), 1.0);
    ///
    /// ```
    pub fn push(&mut self, value: f32) {
        let len = self.data.len();
        self.data[self.pos % len] = value;
    }

    /// Gets the value `self.len` samples prior. Must be called with `push`.
    /// See `push` for examples
    pub fn pop(&mut self) -> f32 {
        let len = self.data.len();
        self.data[(self.pos + 1) % len]
    }

    /// Advances the position of this loop buffer.
    pub fn advance(&mut self) {
        self.pos += 1;
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LowPassFilter {
    /// 1 / cutoff frequency
    pub delay: f32,
    #[serde(skip)]
    pub alpha: f32,
    #[serde(skip)]
    pub last: f32,
}

impl LowPassFilter {
    pub fn new(freq: f32, samples_per_second: u32) -> LowPassFilter {
        LowPassFilter {
            delay: 1.0 / freq,
            alpha: (PI2F * (1.0 / samples_per_second as f32) * freq)
                / (PI2F * (1.0 / samples_per_second as f32) * freq + 1.0),
            last: 0.0,
        }
    }

    #[inline]
    pub fn get_freq(&self) -> f32 {
        1.0 / self.delay
    }

    pub fn filter(&mut self, sample: f32) -> f32 {
        let ret = (sample - self.last).mul_add(self.alpha, self.last);
        self.last = ret;
        ret
    }

    pub fn get_changed(&mut self, freq: f32, samples_per_second: u32) -> Option<Self> {
        Some(Self::new(freq, samples_per_second))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DelayLine {
    pub samples: LoopBuffer,
}

impl DelayLine {
    pub fn new(delay: usize, samples_per_second: u32) -> DelayLine {
        DelayLine {
            samples: LoopBuffer::new(delay, samples_per_second),
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
    /*if 0.0 < crank_pos && crank_pos < ignition_time {
        (PI2F * (crank_pos * ignition_time + 0.5)).sin()
    } else {
        0.0
    }*/
    if 0.5 < crank_pos && crank_pos < ignition_time / 2.0 + 0.5 {
        (PI2F * ((crank_pos - 0.5) / ignition_time)).sin()
    } else {
        0.0
    }
}
