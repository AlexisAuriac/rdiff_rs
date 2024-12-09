#[derive(Debug)]
pub struct Rollsum {
    count: usize,
    s1: u16,
    s2: u16,
}

const ROLLSUM_CHAR_OFFSET: u16 = 31;

impl Rollsum {
    pub fn new() -> Self {
        return Rollsum {
            count: 0,
            s1: 0,
            s2: 0,
        };
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn update(&mut self, p: &[u8]) {
        let l = p.len();

        let mut n = 0;
        while n < l {
            // weird optimisation here based on ref impl, not sure if it makes sense here
            if n + 15 < l {
                for i in 0..16 {
                    self.s1 = self.s1.wrapping_add(p[n + i] as u16);
                    self.s2 = self.s2.wrapping_add(self.s1);
                }
                n += 16;
            } else {
                self.s1 = self.s1.wrapping_add(p[n] as u16);
                self.s2 = self.s2.wrapping_add(self.s1);
                n += 1;
            }
        }

        self.s1 = self
            .s1
            .wrapping_add((l as u16).wrapping_mul(ROLLSUM_CHAR_OFFSET));
        // don't think `(l+1)*l/2` should be allowed to overflow, but not sure,
        // the maximum allowed value of l is not clear either
        // ref: https://github.com/librsync/librsync/blob/ee3df5c8775d571871170c52613c36af1a51db36/src/rollsum.c#L31
        self.s2 = self
            .s2
            .wrapping_add((((l + 1) * l / 2) as u16).wrapping_mul(ROLLSUM_CHAR_OFFSET));
        self.count += l;
    }

    pub fn rotate(&mut self, outb: u8, inb: u8) {
        self.s1 = self.s1.wrapping_add((inb as u16).wrapping_sub(outb as u16));
        self.s2 = self.s2.wrapping_add(self.s1.wrapping_sub(
            (self.count as u16).wrapping_mul((outb as u16).wrapping_add(ROLLSUM_CHAR_OFFSET)),
        ));
    }

    pub fn roll_in(&mut self, inb: u8) {
        self.s1 = self
            .s1
            .wrapping_add(inb as u16)
            .wrapping_add(ROLLSUM_CHAR_OFFSET);
        self.s2 = self.s2.wrapping_add(self.s1);
        self.count += 1;
    }

    pub fn roll_out(&mut self, outb: u8) {
        self.s1 = self
            .s1
            .wrapping_sub(outb as u16)
            .wrapping_sub(ROLLSUM_CHAR_OFFSET);
        self.s2 = self
            .s2
            .wrapping_sub((self.count as u16).wrapping_mul(outb as u16 + ROLLSUM_CHAR_OFFSET));
        self.count -= 1;
    }

    pub fn digest(&self) -> u32 {
        return ((self.s2 as u32) << 16) | ((self.s1 as u32) & 0xffff);
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.s1 = 0;
        self.s2 = 0;
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

        r.roll_in(222);
        assert_eq!(0x00FD00FD, r.digest());
        r.roll_in(11);
        assert_eq!(0x02240127, r.digest());
        r.roll_in(0);
        assert_eq!(0x036A0146, r.digest());
        r.roll_in(13);
        assert_eq!(0x04DC0172, r.digest());
        r.roll_in(7);
        assert_eq!(0x06740198, r.digest());

        r.roll_out(222);
        assert_eq!(0x0183009B, r.digest());
        r.roll_out(11);
        assert_eq!(0x00DB0071, r.digest());
        r.roll_out(0);
        assert_eq!(0x007E0052, r.digest());

        r.roll_in(1);
        assert_eq!(0x00F00072, r.digest());
    }

    #[test]
    fn update() {
        let mut r = Rollsum::new();
        let data = [222, 11, 0, 13, 7];
        let more_data = [66, 171, 8];

        r.update(&data);
        assert_eq!(0x06740198, r.digest());
        r.update(&more_data);
        assert_eq!(0x0E1A02EA, r.digest())
    }

    #[test]
    fn rotate() {
        let mut r = Rollsum::new();
        let data = [222, 11, 0, 13, 7];

        r.update(&data);

        r.rotate(222, 39);
        assert_eq!(0x026400E1, r.digest());
        r.rotate(11, 177);
        assert_eq!(0x03190187, r.digest());
        r.rotate(0, 0);
        assert_eq!(0x04050187, r.digest());
    }

    #[test]
    fn consistency() {
        let data1 = [66, 1, 111, 54, 171, 12, 255, 199, 1, 2, 7, 12, 54, 43, 101];
        let data2 = [4, 22, 66, 1, 111, 54, 171, 12, 255, 199, 1, 2, 7, 12, 54];

        let mut rk1 = Rollsum::new();
        rk1.update(&data1);

        let mut rk2 = Rollsum::new();
        for v in data2 {
            rk2.roll_in(v);
        }
        rk2.rotate(4, 43);
        rk2.roll_out(22);
        rk2.roll_in(101);

        assert_eq!(rk1.digest(), rk2.digest());
    }

    #[test]
    fn rotate_byte_subtraction_bug() {
        let mut rk1 = Rollsum::new();
        rk1.roll_in(1);
        assert_eq!(0x00200020, rk1.digest());

        let mut rk2 = Rollsum::new();
        rk2.roll_in(2);
        rk2.rotate(2, 1);

        assert_eq!(rk1.digest(), rk2.digest());
    }
}
