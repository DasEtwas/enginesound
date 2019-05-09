use crate::{gen::LoopBuffer, SAMPLE_RATE};
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
