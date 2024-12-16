// based on librsync implementation
// https://github.com/librsync/librsync/blob/ee3df5c8775d571871170c52613c36af1a51db36/src/rabinkarp.c

use std::num::Wrapping;

#[derive(Debug)]
pub struct RabinKarp {
    count: usize,
    hash: Wrapping<u32>,
    mult: Wrapping<u32>,
}

const RABINKARP_SEED: u32 = 1;
const RABINKARP_MULT: u32 = 0x08104225;
const RABINKARP_INVM: u32 = 0x98f009ad;
const RABINKARP_ADJ: u32 = 0x08104224;

static RABINKARP_MULT_POW2: [u32; 32] = [
    0x08104225, 0xa5b71959, 0xf9c080f1, 0x7c71e2e1, 0x0bb409c1, 0x4dc72381, 0xd17a8701, 0x96260e01,
    0x55101c01, 0x2d303801, 0x66a07001, 0xfe40e001, 0xc081c001, 0x91038001, 0x62070001, 0xc40e0001,
    0x881c0001, 0x10380001, 0x20700001, 0x40e00001, 0x81c00001, 0x03800001, 0x07000001, 0x0e000001,
    0x1c000001, 0x38000001, 0x70000001, 0xe0000001, 0xc0000001, 0x80000001, 0x00000001, 0x00000001,
];

#[inline]
fn rabinkarp_pow(mut n: u32) -> u32 {
    let mut ans = 1;

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

    ans
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

    pub fn update(&mut self, p: &[u8]) {
        for b in p {
            self.hash = Wrapping(RABINKARP_MULT) * self.hash + Wrapping(*b as u32);
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
