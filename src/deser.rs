use crate::{gen::LoopBuffer, SAMPLE_RATE};
use serde::*;
use simdeez::{avx2::*, scalar::*, sse2::*, sse41::*, *};

#[derive(Deserialize)]
pub struct LoopBufferDeser {
    // in seconds
    pub delay: f32,
}

impl From<LoopBufferDeser> for LoopBuffer {
    fn from(from: LoopBufferDeser) -> Self {
        simd_runtime_generate!(
            fn get_best_simd_size(size: usize) -> usize {
                ((size - 1) / S::VF32_WIDTH + 1) * S::VF32_WIDTH
            }
        );
        let len = (from.delay * SAMPLE_RATE as f32) as usize;
        let bufsize = get_best_simd_size_runtime_select(len);

        LoopBuffer {
            delay: from.delay,
            len,
            data: vec![0.0; bufsize],
            pos: 0,
        }
    }
}
