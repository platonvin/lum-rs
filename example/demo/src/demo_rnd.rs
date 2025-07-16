use std::{
    num::NonZeroU64,
    random::{Distribution, RandomSource},
};

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
}

impl RandomSource for JustFuckingRandomGenerator {
    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        self.fill_bytes(bytes);
    }
}

pub struct JustFuckingDistributionU64 {}

impl Distribution<u64> for JustFuckingDistributionU64 {
    fn sample(&self, source: &mut (impl RandomSource + ?Sized)) -> u64 {
        let mut bytes = [0u8; 8];
        source.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }
}
