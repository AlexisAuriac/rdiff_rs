use std::num::Wrapping;

#[derive(Debug)]
pub struct Rollsum {
    count: usize,
    s1: Wrapping<u16>,
    s2: Wrapping<u16>,
}

const ROLLSUM_CHAR_OFFSET: u16 = 31;

impl Rollsum {
    #[inline]
    pub fn new() -> Self {
        Rollsum {
            count: 0,
            s1: Wrapping(0),
            s2: Wrapping(0),
        }
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn update(&mut self, p: &[u8]) {
        let l = p.len();

        for b in p {
            self.s1 += *b as u16;
            self.s2 += self.s1;
        }

        self.s1 += Wrapping(l as u16) * Wrapping(ROLLSUM_CHAR_OFFSET);
        // don't think `(l+1)*l/2` should be allowed to overflow, but not sure,
        // the maximum allowed value of l is not clear either
        // ref: https://github.com/librsync/librsync/blob/ee3df5c8775d571871170c52613c36af1a51db36/src/rollsum.c#L31
        self.s2 += Wrapping(((l + 1) * l / 2) as u16) * Wrapping(ROLLSUM_CHAR_OFFSET);
        self.count += l;
    }

    #[inline]
    pub fn rotate(&mut self, outb: u8, inb: u8) {
        debug_assert!(self.count > 0, "rotate on an empty rollsum");

        self.s1 += Wrapping(inb as u16) - Wrapping(outb as u16);
        self.s2 +=
            self.s1 - (Wrapping(self.count as u16) * Wrapping(outb as u16 + ROLLSUM_CHAR_OFFSET));
    }

    #[inline]
    pub fn rollin(&mut self, inb: u8) {
        self.s1 += inb as u16 + ROLLSUM_CHAR_OFFSET;
        self.s2 += self.s1;
        self.count += 1;
    }

    #[inline]
    pub fn rollout(&mut self, outb: u8) {
        debug_assert!(self.count > 0, "rollout on an empty rollsum");

        self.s1 -= outb as u16 + ROLLSUM_CHAR_OFFSET;
        self.s2 -= Wrapping(self.count as u16) * Wrapping(outb as u16 + ROLLSUM_CHAR_OFFSET);
        self.count -= 1;
    }

    #[inline]
    pub fn digest(&self) -> u32 {
        ((self.s2.0 as u32) << 16) | ((self.s1.0 as u32) & 0xffff)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.count = 0;
        self.s1 = Wrapping(0);
        self.s2 = Wrapping(0);
    }
}

impl Default for Rollsum {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let r = Rollsum::new();
        assert_eq!(0, r.digest());
    }

    #[test]
    fn rollin_rollout() {
        let mut r = Rollsum::new();

        r.rollin(222);
        assert_eq!(0x00FD00FD, r.digest());
        r.rollin(11);
        assert_eq!(0x02240127, r.digest());
        r.rollin(0);
        assert_eq!(0x036A0146, r.digest());
        r.rollin(13);
        assert_eq!(0x04DC0172, r.digest());
        r.rollin(7);
        assert_eq!(0x06740198, r.digest());

        r.rollout(222);
        assert_eq!(0x0183009B, r.digest());
        r.rollout(11);
        assert_eq!(0x00DB0071, r.digest());
        r.rollout(0);
        assert_eq!(0x007E0052, r.digest());

        r.rollin(1);
        assert_eq!(0x00F00072, r.digest());
    }

    #[test]
    fn update() {
        let mut r = Rollsum::new();
        let data = [222, 11, 0, 13, 7];
        let more_data = [66, 171, 8];

        r.update(&data);
        assert_eq!(0x06740198, r.digest());
        assert_eq!(r.count(), data.len());

        r.update(&more_data);
        assert_eq!(0x0E1A02EA, r.digest());
        assert_eq!(r.count(), data.len() + more_data.len());
    }

    #[test]
    fn rotate() {
        let mut r = Rollsum::new();
        let data = [222, 11, 0, 13, 7];

        r.update(&data);
        let init_count = r.count();

        r.rotate(222, 39);
        assert_eq!(0x026400E1, r.digest());
        assert_eq!(r.count(), init_count);

        r.rotate(11, 177);
        assert_eq!(0x03190187, r.digest());
        assert_eq!(r.count(), init_count);

        r.rotate(0, 0);
        assert_eq!(0x04050187, r.digest());
        assert_eq!(r.count(), init_count);
    }

    #[test]
    fn consistency() {
        let data1 = [66, 1, 111, 54, 171, 12, 255, 199, 1, 2, 7, 12, 54, 43, 101];
        let data2 = [4, 22, 66, 1, 111, 54, 171, 12, 255, 199, 1, 2, 7, 12, 54];

        let mut rk1 = Rollsum::new();
        rk1.update(&data1);

        let mut rk2 = Rollsum::new();
        for v in data2 {
            rk2.rollin(v);
        }
        rk2.rotate(4, 43);
        rk2.rollout(22);
        rk2.rollin(101);

        assert_eq!(rk1.digest(), rk2.digest());
    }

    #[test]
    fn rotate_byte_subtraction_bug() {
        let mut rk1 = Rollsum::new();
        rk1.rollin(1);
        assert_eq!(0x00200020, rk1.digest());

        let mut rk2 = Rollsum::new();
        rk2.rollin(2);
        rk2.rotate(2, 1);

        assert_eq!(rk1.digest(), rk2.digest());
    }
}
