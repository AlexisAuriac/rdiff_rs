use std::{
    io::{self, Read},
    thread::sleep,
    time::Duration,
};

pub struct SlowReader<R> {
    r: R,
    iter: Box<dyn Iterator<Item = Duration>>,
    total: Duration,
}

impl<R> SlowReader<R>
where
    R: Read,
{
    pub fn with_interval(r: R, d: Duration) -> Self {
        let iter = [d].into_iter().cycle();

        Self::with_iter(r, Box::new(iter))
    }

    pub fn with_iter(r: R, iter: Box<dyn Iterator<Item = Duration>>) -> Self {
        Self {
            r,
            iter,
            total: Duration::new(0, 0),
        }
    }

    pub fn total_waited(&self) -> Duration {
        self.total
    }
}

impl<R> Read for SlowReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let wait = match self.iter.next() {
            None => return Ok(0),
            Some(wait) => wait,
        };

        sleep(wait);
        self.total += wait;

        self.r.read(buf)
    }
}
