use std::hash::Hasher;

pub struct Fnv1a64 {
    hash: u64,
}

impl Fnv1a64 {
    pub fn new() -> Self {
        Self {
            hash: 0xcbf29ce484222325,
        }
    }
}

impl Hasher for Fnv1a64 {
    fn write(&mut self, bytes: &[u8]) {
        const PRIME: u64 = 0x100000001b3;

        for &byte in bytes {
            self.hash ^= byte as u64;
            self.hash = self.hash.wrapping_mul(PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}
