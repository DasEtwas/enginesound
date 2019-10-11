use crate::{
    gen::{LoopBuffer, LowPassFilter},
    SAMPLE_RATE,
};
use serde::*;

#[derive(Deserialize)]
pub struct LoopBufferDeser {
    // in seconds
    pub delay: f32,
}

impl From<LoopBufferDeser> for LoopBuffer {
    fn from(from: LoopBufferDeser) -> Self {
        let len = (from.delay * SAMPLE_RATE as f32) as usize;
        let bufsize = LoopBuffer::get_best_simd_size(len);

        LoopBuffer {
            delay: from.delay,
            len,
            data: vec![0.0; bufsize],
            pos: 0,
        }
    }
}

#[derive(Deserialize)]
pub struct LowPassFilterDeser {
    // in seconds
    pub delay: f32,
}

impl From<LowPassFilterDeser> for LowPassFilter {
    fn from(from: LowPassFilterDeser) -> Self {
        let len = (from.delay * SAMPLE_RATE as f32)
            .min(SAMPLE_RATE as f32)
            .max(1.0);
        LowPassFilter {
            samples: LoopBuffer::new(len.ceil() as usize, SAMPLE_RATE),
            delay: from.delay,
            len,
        }
    }
}
