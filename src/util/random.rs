pub struct XORShift {
    state: u64,
}

impl XORShift {
    pub fn new(seed: u64) -> XORShift {
        XORShift { state: seed }
    }

    pub fn sample(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;

        return self.state.wrapping_mul(0x2545F4914F6CDD1D);
    }
}
