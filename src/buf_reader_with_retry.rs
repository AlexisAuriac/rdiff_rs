use std::io::{self, BufReader, Read};

pub struct BufReaderWithRetry<R: Read> {
    inner: BufReader<R>,
}

// BufReaderWithRetry Makes it easier to pull blocks of a specific size from a reader
impl<R: Read> BufReaderWithRetry<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner: BufReader::new(inner),
        }
    }

    pub fn with_capacity(cap: usize, inner: R) -> Self {
        Self {
            inner: BufReader::with_capacity(cap, inner),
        }
    }
}

// When BufReader runs out of data it will give us what it has alread buffered but won't get more
// data until the next read call, this doesn't work for us so we retry if it gives us a partial block
impl<R: Read> Read for BufReaderWithRetry<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total = self.inner.read(buf)?;
        if total == 0 || total == buf.len() {
            return Ok(total);
        }

        while total < buf.len() {
            let n = self.inner.read(&mut buf[total..])?;
            if n == 0 {
                break;
            }

            total += n;
        }

        return Ok(total);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use anyhow::Error;

    use super::*;

    #[test]
    fn bufreader_retry() -> Result<(), Error> {
        let data = vec![0, 1, 2, 3, 4];
        let cursor = Cursor::new(data);
        let mut buf_r = BufReaderWithRetry::with_capacity(3, cursor);

        let mut output = [0u8; 2];
        let n = buf_r.read(&mut output[..])?;
        assert_eq!(n, 2);
        assert_eq!(output, [0, 1]);

        let mut output = [0u8; 2];
        let n = buf_r.read(&mut output[..])?;
        assert_eq!(n, 2); // BufReader would return 1
        assert_eq!(output, [2, 3]); // BufReader would return [2, 0]

        let mut output = [0u8; 2];
        let n = buf_r.read(&mut output[..])?;
        assert_eq!(n, 1);
        assert_eq!(output, [4, 0]);

        let mut output = [0u8; 2];
        let n = buf_r.read(&mut output[..])?;
        assert_eq!(n, 0);
        assert_eq!(output, [0, 0]);

        Ok(())
    }
}
