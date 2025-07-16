use std::{num::NonZeroU64, random::RandomSource};

pub struct JustFuckingRandomGenerator {
    state: u64,
}

impl JustFuckingRandomGenerator {
    // zero does not work for this algorithm
    pub fn new(seed: NonZeroU64) -> Self {
        Self {
            state: u64::from(seed),
        }
    }

    /// xorshift64
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn fill_bytes(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let rnd = self.next_u64().to_le_bytes();
            let len = chunk.len();
            chunk.copy_from_slice(&rnd[..len]);
        }
    }

    fn next_f32(&mut self) -> f32 {
        let r = (self.next_u64() >> 40) as u32;
        (r as f32) / (u32::MAX >> 8) as f32
    }
}

impl RandomSource for JustFuckingRandomGenerator {
    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        self.fill_bytes(bytes);
    }
}
