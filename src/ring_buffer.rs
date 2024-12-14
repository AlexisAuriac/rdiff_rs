// largely based on https://github.com/armon/circbuf

use std::cmp::Ordering;

#[derive(Debug)]
pub struct RingBuffer {
    size: usize,
    cursor: usize,
    written: usize,
    data: Vec<u8>,
}

impl RingBuffer {
    #[inline]
    pub fn new(size: usize) -> Self {
        RingBuffer {
            size,
            cursor: 0,
            written: 0,
            data: vec![0u8; size],
        }
    }

    #[inline]
    pub fn total_written(&self) -> usize {
        self.written
    }

    pub fn as_bytes(&mut self) -> &[u8] {
        if self.written > self.size {
            self.data.rotate_left(self.cursor);
            self.cursor = 0;
        }

        if self.written < self.size {
            &self.data[..self.written]
        } else {
            &self.data
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.cursor = 0;
        self.written = 0;
    }

    pub fn write(&mut self, mut buf: &[u8]) -> usize {
        let n = buf.len();
        self.written += n;

        if n > self.size {
            buf = &buf[n - self.size..];
        }

        let remain = self.size - self.cursor;

        match remain.cmp(&buf.len()) {
            Ordering::Equal => self.data[self.cursor..].copy_from_slice(buf),
            Ordering::Greater => {
                let start = self.cursor;
                let end = self.cursor + buf.len();

                self.data[start..end].copy_from_slice(buf);
            }
            Ordering::Less => {
                let (left_out, right_out) = self.data.split_at_mut(self.cursor);
                let (left_in, right_in) = buf.split_at(remain);

                right_out.copy_from_slice(left_in);
                left_out[..buf.len() - remain].copy_from_slice(right_in);
            }
        }

        self.cursor = (self.cursor + buf.len()) % self.size;
        n
    }

    #[inline]
    pub fn write_byte(&mut self, b: u8) {
        self.data[self.cursor] = b;
        self.cursor = (self.cursor + 1) % self.size;
        self.written += 1;
    }

    pub fn get(&self, i: usize) -> Option<u8> {
        if i >= self.written || i >= self.size {
            None
        } else if self.written > self.size {
            Some(self.data[(self.cursor + i) % self.size])
        } else {
            Some(self.data[i])
        }
    }

    #[inline(never)]
    pub fn front(&self) -> Option<u8> {
        if self.written == 0 {
            None
        } else {
            Some(self.data[self.cursor])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_write() {
        let mut buf = RingBuffer::new(1024);

        let inp = b"hello world";
        let n = buf.write(inp);

        assert_eq!(n, inp.len());
        assert_eq!(buf.as_bytes(), inp);
    }

    #[test]
    fn full_write() {
        let inp = b"hello world";
        let mut buf = RingBuffer::new(inp.len());

        let n = buf.write(inp);

        assert_eq!(n, inp.len());
        assert_eq!(buf.as_bytes(), inp);
    }

    #[test]
    fn long_write() {
        let mut buf = RingBuffer::new(6);
        let inp = b"hello world";

        let n = buf.write(inp);

        assert_eq!(n, inp.len());
        assert_eq!(buf.as_bytes(), b" world");
    }

    #[test]
    fn huge_write() {
        let mut buf = RingBuffer::new(3);
        let inp = b"hello world";

        let n = buf.write(inp);

        assert_eq!(n, inp.len());
        assert_eq!(buf.as_bytes(), b"rld");
    }

    #[test]
    fn many_small() {
        let mut buf = RingBuffer::new(3);
        let inp = b"hello world";

        for b in inp {
            let n = buf.write(&[*b]);
            assert_eq!(n, 1);
        }

        assert_eq!(buf.as_bytes(), b"rld");
    }

    #[test]
    fn multi_part() {
        let mut buf = RingBuffer::new(16);
        let inps = [
            &b"hello world\n"[..],
            &b"this is a test\n"[..],
            &b"my cool input\n"[..],
        ];

        let mut total = 0;
        for inp in inps {
            total += inp.len();

            let n = buf.write(inp);
            assert_eq!(n, inp.len());
        }

        assert_eq!(buf.total_written(), total);
        assert_eq!(buf.as_bytes(), b"t\nmy cool input\n");
    }

    #[test]
    fn reset() {
        let mut buf = RingBuffer::new(4);
        let inps = [
            &b"hello world\n"[..],
            &b"this is a test\n"[..],
            &b"my cool input\n"[..],
        ];

        for inp in inps {
            let n = buf.write(inp);
            assert_eq!(n, inp.len());
        }

        buf.reset();

        let inp = b"hello";

        let n = buf.write(inp);
        assert_eq!(n, inp.len());
        assert_eq!(buf.as_bytes(), b"ello");
    }

    #[test]
    fn write_byte() {
        let inp = b"hello world";
        let mut buf = RingBuffer::new(4);

        for b in inp {
            buf.write_byte(*b);
        }

        assert_eq!(buf.as_bytes(), b"orld");
    }

    #[test]
    fn get() {
        let t = |inp: &[u8]| {
            let init_data = b"hell";
            let mut buf = RingBuffer::new(inp.len());

            buf.write(init_data);
            for (i, b) in init_data.iter().enumerate() {
                let b2 = buf.get(i).expect("index out of bounds");
                assert_eq!(b2, *b);
            }

            buf.write(inp);

            for (i, b) in inp.iter().enumerate() {
                let b2 = buf.get(i).expect("index out of bounds");
                assert_eq!(b2, *b);
            }
        };

        t(b"hello world");
        t(b"hey, hello world");
    }
}
