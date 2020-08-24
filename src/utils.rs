use crate::gen::{Engine, LoopBuffer, LowPassFilter};
use std::fs::File;

pub const SPEED_OF_SOUND: f32 = 343.0; // m/s

/// converts a given amount of time into samples
pub fn seconds_to_samples(seconds: f32, sample_rate: u32) -> usize {
    (seconds * sample_rate as f32).max(1.0) as usize
}

/// converts a given distance into samples via the speed of sound
pub fn distance_to_samples(meters: f32, sample_rate: u32) -> usize {
    seconds_to_samples(meters / SPEED_OF_SOUND, sample_rate)
}

pub fn samples_to_seconds(samples: usize, sample_rate: u32) -> f32 {
    samples as f32 / sample_rate as f32
}

/// returns meters
pub fn samples_to_distance(samples: usize, sample_rate: u32) -> f32 {
    samples_to_seconds(samples, sample_rate) * SPEED_OF_SOUND
}

pub(crate) fn load_engine(path: &str, sample_rate: u32) -> Result<Engine, String> {
    match File::open(path) {
        Ok(file) => match ron::de::from_reader::<_, Engine>(file) {
            Ok(mut engine) => {
                fix_engine(&mut engine, sample_rate);
                Ok(engine)
            }
            Err(e) => Err(format!("Failed to load config \"{}\": {}", &path, e)),
        },
        Err(e) => Err(format!("Failed to open file \"{}\": {}", &path, e)),
    }
}

pub fn fix_engine(engine: &mut Engine, sample_rate: u32) {
    fn fix_lpf(lpf: &mut LowPassFilter, sample_rate: u32) {
        *lpf = LowPassFilter::new(1.0 / lpf.delay, sample_rate);
    }

    fn fix_loop_buffer(lb: &mut LoopBuffer, sample_rate: u32) {
        let len = (lb.delay * sample_rate as f32) as usize;

        *lb = LoopBuffer {
            delay: lb.delay,
            data: vec![0.0; len],
            pos: 0,
        };
    }

    vec![
        &mut engine.crankshaft_fluctuation_lp,
        &mut engine.engine_vibration_filter,
        &mut engine.intake_noise_lp,
    ]
    .into_iter()
    .for_each(|lpf| fix_lpf(lpf, sample_rate));

    engine
        .muffler
        .muffler_elements
        .iter_mut()
        .chain(std::iter::once(&mut engine.muffler.straight_pipe))
        .flat_map(|waveguide| vec![&mut waveguide.chamber0, &mut waveguide.chamber1].into_iter())
        .chain(engine.cylinders.iter_mut().flat_map(|cylinder| {
            vec![
                &mut cylinder.exhaust_waveguide.chamber0,
                &mut cylinder.exhaust_waveguide.chamber1,
                &mut cylinder.extractor_waveguide.chamber0,
                &mut cylinder.extractor_waveguide.chamber1,
                &mut cylinder.intake_waveguide.chamber0,
                &mut cylinder.intake_waveguide.chamber1,
            ]
            .into_iter()
        }))
        .for_each(|delay_line| fix_loop_buffer(&mut delay_line.samples, sample_rate));
}
