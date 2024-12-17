// based on librsync implementation
// https://github.com/librsync/librsync/blob/ee3df5c8775d571871170c52613c36af1a51db36/src/rabinkarp.c

use std::num::Wrapping;

use crate::weak_sum::rabin_karp_consts::RABINKARP_MULT_POW2;

use super::rabin_karp_consts::{
    RABINKARP_ADJ, RABINKARP_INVM, RABINKARP_MULT, RABINKARP_MULT_POW, RABINKARP_SEED,
};

#[derive(Debug)]
pub struct RabinKarp {
    count: usize,
    hash: Wrapping<u32>,
    mult: Wrapping<u32>,
}

#[inline]
fn rabinkarp_pow(mut n: u32) -> u32 {
    let mut ans = Wrapping(1);

    let mut i = 0;
    while n != 0 {
        debug_assert!(
            i < RABINKARP_MULT_POW2.len(),
            "outside of RABINKARP_MULT_POW2 array"
        );

        if n & 1 != 0 {
            ans *= RABINKARP_MULT_POW2[i];
        }
        i += 1;
        n >>= 1;
    }

    ans.0
}

impl RabinKarp {
    #[inline]
    pub fn new() -> Self {
        Self {
            count: 0,
            hash: Wrapping(RABINKARP_SEED),
            mult: Wrapping(1),
        }
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    // hash_n = hash_n-1 * m + x_n-1 <--- simple but inefficient
    // ---
    // hash_1 = hash_0 * m + x0
    // ---
    // hash_2 = hash1 * m + x1
    // hash_2 = (hash_0 * m + x0) * m + x1
    // hash_2 = hash_0 * m^2 + x0 * m + x1
    // ---
    // hash_3 = hash2 * m + x2
    // hash_3 = (hash_0 * m^2 + x0 * m + x1) * m + x2
    // hash_3 = hash_0 * m^3 + x0 * m^2 + x1 * m + x2
    // --- therefore
    // hash_n = hash_0 * m^n + sum(x_i * m^(n-i) for x in 0..n)
    // --> less math operations + we can precompute powers of m
    pub fn update(&mut self, p: &[u8]) {
        // divide the buffer into chunks so we can keep using precomputed value
        // even if p.len() is very large
        // rchunks is a lot faster than chunks here, and we don't care about order
        for chunk in p.rchunks(RABINKARP_MULT_POW.len()) {
            let mut m = 1;
            let mut tmp_hash = Wrapping(0);

            for (i, b) in chunk.iter().rev().enumerate() {
                tmp_hash += Wrapping(*b as u32) * Wrapping(m);
                m = RABINKARP_MULT_POW[i];
            }

            self.hash = self.hash * Wrapping(m) + tmp_hash;
        }

        self.count += p.len();
        self.mult *= rabinkarp_pow(p.len() as u32);
    }

    #[inline]
    pub fn rotate(&mut self, outb: u8, inb: u8) {
        self.hash = self.hash * Wrapping(RABINKARP_MULT) + Wrapping(inb as u32)
            - self.mult * Wrapping(outb as u32 + RABINKARP_ADJ);
    }

    #[inline]
    pub fn rollin(&mut self, inb: u8) {
        self.hash = self.hash * Wrapping(RABINKARP_MULT) + Wrapping(inb as u32);
        self.count += 1;
        self.mult *= RABINKARP_MULT;
    }

    #[inline]
    pub fn rollout(&mut self, outb: u8) {
        self.count -= 1;
        self.mult *= RABINKARP_INVM;
        self.hash -= self.mult * (Wrapping(outb as u32) + Wrapping(RABINKARP_ADJ));
    }

    #[inline]
    pub fn digest(&self) -> u32 {
        self.hash.0
    }

    #[inline]
    pub fn reset(&mut self) {
        self.count = 0;
        self.hash = Wrapping(RABINKARP_SEED);
        self.mult = Wrapping(1);
    }
}

impl Default for RabinKarp {
    fn default() -> Self {
        RabinKarp::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let r = RabinKarp::new();

        assert_eq!(r.count(), 0);
        assert_eq!(r.digest(), 0x00000001);
    }

    #[test]
    fn rollin() {
        let mut r = RabinKarp::new();

        r.rollin(0);
        assert_eq!(r.count(), 1);
        assert_eq!(r.digest(), 0x08104225);

        r.rollin(1);
        r.rollin(2);
        r.rollin(3);
        assert_eq!(r.count(), 4);
        assert_eq!(r.digest(), 0xaf981e97);
    }

    #[test]
    fn rotate() {
        let mut r = RabinKarp::new();
        let init_data = [0, 1, 2, 3];

        for b in init_data {
            r.rollin(b);
        }

        r.rotate(0, 4);
        assert_eq!(r.count, 4);
        assert_eq!(r.digest(), 0xe2ef15f3);

        r.rotate(1, 5);
        r.rotate(2, 6);
        r.rotate(3, 7);
        assert_eq!(r.count, 4);
        assert_eq!(r.digest(), 0x7cf3fc07);
    }

    #[test]
    fn rollout() {
        let mut r = RabinKarp::new();
        let init_data = [4, 5, 6, 7];

        for b in init_data {
            r.rollin(b);
        }
        assert_eq!(r.digest(), 0x7cf3fc07);

        r.rollout(4);
        assert_eq!(r.count, 3);
        assert_eq!(r.digest(), 0xf284a77f);

        r.rollout(5);
        r.rollout(6);
        r.rollout(7);
        assert_eq!(r.count, 0);
        assert_eq!(r.digest(), 0x00000001);
    }

    #[test]
    fn update() {
        let mut r = RabinKarp::new();
        let buf = (0..=255).collect::<Vec<u8>>();

        r.update(&buf);
        assert_eq!(r.digest(), 0xc1972381);
    }
}
