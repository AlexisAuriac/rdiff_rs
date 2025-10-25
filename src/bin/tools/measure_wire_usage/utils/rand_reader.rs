use std::io::{self, Read};

use rand::{RngCore, SeedableRng, rngs::StdRng};

pub struct RandReader {
    rng: StdRng,
}

impl RandReader {
    pub fn seed_from_u64(state: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(state),
        }
    }
}

impl Read for RandReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.rng.fill_bytes(buf);
        Ok(buf.len())
    }
}
