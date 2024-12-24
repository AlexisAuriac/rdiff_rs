use std::io::{self, Write};

pub struct CountWriter<W> {
    pub count: usize,
    w: W,
}

impl<W> CountWriter<W>
where
    W: Write,
{
    pub fn new(w: W) -> Self {
        Self { count: 0, w }
    }
}

impl<W> Write for CountWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.w.write(buf)?;
        self.count += n;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}
