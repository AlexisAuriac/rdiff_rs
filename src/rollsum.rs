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
        self.s1 = self.s1.wrapping_add(inb.wrapping_sub(outb) as u16);
        self.s2 = self.s2.wrapping_add(self.s1);
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
