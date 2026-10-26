/// A small seeded pseudo-random generator (SplitMix64).
///
/// It is written out here rather than taken from a crate so that a seed gives the same
/// sequence on every platform and every version of the dependency tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value uniformly distributed in `[0, 1)`.
    pub fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A value uniformly distributed in `[-amplitude, amplitude)`.
    pub fn uniform(&mut self, amplitude: f64) -> f64 {
        (2.0 * self.next_unit() - 1.0) * amplitude
    }
}
