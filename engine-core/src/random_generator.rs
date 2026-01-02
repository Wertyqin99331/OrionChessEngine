pub(crate) struct XorShift64Star {
    state: u64,
}

impl XorShift64Star {
    const DEFAULT_STATE: u64 = 0x9e3779b97f4a7c15;

    pub(crate) const fn new() -> Self {
        XorShift64Star::with_seed(XorShift64Star::DEFAULT_STATE)
    }

    pub(crate) const fn with_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(crate) const fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub(crate) const fn generate_magic_number_candidate(&mut self) -> u64 {
        self.next_u64() & self.next_u64() & self.next_u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_xor_shift_64_star_random_generator() {
        let mut rnd_generator = XorShift64Star::new();
        for i in 1..=100 {
            println!("{i} {}", rnd_generator.next_u64());
        }
    }

    #[test]
    #[ignore]
    fn test_generate_magic_number_candidate() {
        let mut rnd_gen = XorShift64Star::new();

        for i in 0..10 {
            println!("{i} {}", rnd_gen.generate_magic_number_candidate());
        }
    }
}
