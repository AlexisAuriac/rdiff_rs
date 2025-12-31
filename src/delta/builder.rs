use std::io::Write;

use crate::{
    error::Error,
    op::{Op, OpArgLen},
};

pub const DELTA_MAGIC: u32 = 0x72730236;

const DEFAULT_OUTPUT_BUFFER_SIZE: usize = 16 * 1024 * 1024;

fn min_int_size(d: u64) -> OpArgLen {
    if d >= 2u64.pow(32) {
        OpArgLen::N8
    } else if d >= 2u64.pow(16) {
        OpArgLen::N4
    } else if d >= 2u64.pow(8) {
        OpArgLen::N2
    } else {
        OpArgLen::N1
    }
}

fn copy_op_from_arg_size(arg1: OpArgLen, arg2: OpArgLen) -> Op {
    match (arg1, arg2) {
        (OpArgLen::N1, OpArgLen::N1) => Op::CopyN1N1,
        (OpArgLen::N1, OpArgLen::N2) => Op::CopyN1N2,
        (OpArgLen::N1, OpArgLen::N4) => Op::CopyN1N4,
        (OpArgLen::N1, OpArgLen::N8) => Op::CopyN1N8,
        (OpArgLen::N2, OpArgLen::N1) => Op::CopyN2N1,
        (OpArgLen::N2, OpArgLen::N2) => Op::CopyN2N2,
        (OpArgLen::N2, OpArgLen::N4) => Op::CopyN2N4,
        (OpArgLen::N2, OpArgLen::N8) => Op::CopyN2N8,
        (OpArgLen::N4, OpArgLen::N1) => Op::CopyN4N1,
        (OpArgLen::N4, OpArgLen::N2) => Op::CopyN4N2,
        (OpArgLen::N4, OpArgLen::N4) => Op::CopyN4N4,
        (OpArgLen::N4, OpArgLen::N8) => Op::CopyN4N8,
        (OpArgLen::N8, OpArgLen::N1) => Op::CopyN8N1,
        (OpArgLen::N8, OpArgLen::N2) => Op::CopyN8N2,
        (OpArgLen::N8, OpArgLen::N4) => Op::CopyN8N4,
        (OpArgLen::N8, OpArgLen::N8) => Op::CopyN8N8,
    }
}

fn literal_op_from_arg_size(arg: OpArgLen) -> Op {
    match arg {
        OpArgLen::N1 => Op::LiteralN1,
        OpArgLen::N2 => Op::LiteralN2,
        OpArgLen::N4 => Op::LiteralN4,
        OpArgLen::N8 => Op::LiteralN8,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaSegmentKind {
    Literal,
    Copy,
}

pub struct DeltaBuilder<O> {
    kind: DeltaSegmentKind,
    pos: u64,
    len: u64,
    output: O,
    lit: Vec<u8>,
}

impl<O: Write> DeltaBuilder<O> {
    pub fn with_buffer_size(output: O, buf_size: usize) -> Self {
        assert!(buf_size > 0, "buffer size must be non zero");

        Self {
            output,
            lit: Vec::with_capacity(buf_size),
            kind: DeltaSegmentKind::Literal,
            pos: 0,
            len: 0,
        }
    }

    pub fn new(output: O) -> Self {
        Self::with_buffer_size(output, DEFAULT_OUTPUT_BUFFER_SIZE)
    }

    pub fn write_magic(&mut self) -> Result<(), Error> {
        self.output.write_all(&DELTA_MAGIC.to_be_bytes())?;
        Ok(())
    }

    fn write_op_arg(&mut self, d: u64, size: OpArgLen) -> Result<(), Error> {
        match size {
            OpArgLen::N1 => self.output.write_all(&(d as u8).to_be_bytes())?,
            OpArgLen::N2 => self.output.write_all(&(d as u16).to_be_bytes())?,
            OpArgLen::N4 => self.output.write_all(&(d as u32).to_be_bytes())?,
            OpArgLen::N8 => self.output.write_all(&d.to_be_bytes())?,
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        if self.len == 0 {
            return Ok(());
        }

        match self.kind {
            DeltaSegmentKind::Copy => {
                let pos_size = min_int_size(self.pos);
                let len_size = min_int_size(self.len);
                let op = copy_op_from_arg_size(pos_size, len_size);

                self.output.write_all(&(op as u8).to_be_bytes())?;
                self.write_op_arg(self.pos, pos_size)?;
                self.write_op_arg(self.len, len_size)?;
            }
            DeltaSegmentKind::Literal => {
                let len_size = min_int_size(self.len);
                let op = literal_op_from_arg_size(len_size);

                self.output.write_all(&(op as u8).to_be_bytes())?;
                self.write_op_arg(self.len, len_size)?;
                self.output.write_all(&self.lit)?;
            }
        }

        self.pos = 0;
        self.len = 0;
        self.lit.clear();

        Ok(())
    }

    pub fn copy(&mut self, pos: u64, len: u64) -> Result<(), Error> {
        if self.kind != DeltaSegmentKind::Copy {
            self.flush()?;
            self.kind = DeltaSegmentKind::Copy;
        }

        if self.pos + self.len == pos {
            // segments are contiguous, merge with current working segment
            self.len += len;
        } else {
            self.flush()?;
            self.pos = pos;
            self.len = len;
        }

        Ok(())
    }

    pub fn add_byte(&mut self, b: u8) -> Result<(), Error> {
        if self.kind != DeltaSegmentKind::Literal {
            self.flush()?;
            self.kind = DeltaSegmentKind::Literal;
        } else if self.lit.len() == self.lit.capacity() {
            self.flush()?;
        }

        self.lit.push(b);
        self.len += 1;

        Ok(())
    }

    pub fn add_bytes(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        if self.kind != DeltaSegmentKind::Literal {
            self.flush()?;
            self.kind = DeltaSegmentKind::Literal;
        }

        // make sure we don't go over the buffer capacity
        let mut remain = self.lit.capacity() - self.lit.len();
        while buf.len() > remain {
            let (first, second) = buf.split_at(remain);

            self.lit.extend(first);
            self.len += first.len() as u64;

            self.flush()?;

            buf = second;
            remain = self.lit.capacity();
        }

        self.lit.extend(buf);
        self.len += buf.len() as u64;

        Ok(())
    }

    pub fn end(mut self) -> Result<(), Error> {
        self.output.write_all(&(Op::EndOp as u8).to_be_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_min_int_size() {
        assert_eq!(min_int_size(0), OpArgLen::N1);
        assert_eq!(min_int_size(1), OpArgLen::N1);
        assert_eq!(min_int_size(1 << 7), OpArgLen::N1);
        assert_eq!(min_int_size(u8::MAX as u64), OpArgLen::N1);

        assert_eq!(min_int_size(u8::MAX as u64 + 1), OpArgLen::N2);
        assert_eq!(min_int_size(1 << 8), OpArgLen::N2);
        assert_eq!(min_int_size(u16::MAX as u64), OpArgLen::N2);

        assert_eq!(min_int_size(u16::MAX as u64 + 1), OpArgLen::N4);
        assert_eq!(min_int_size(1 << 31), OpArgLen::N4);
        assert_eq!(min_int_size(u32::MAX as u64), OpArgLen::N4);

        assert_eq!(min_int_size(u32::MAX as u64 + 1), OpArgLen::N8);
        assert_eq!(min_int_size(1 << 32), OpArgLen::N8);
        assert_eq!(min_int_size(u64::MAX), OpArgLen::N8);
    }

    #[test]
    fn add_byte() -> Result<(), Error> {
        let mut buf = Vec::new();
        let w = Cursor::new(&mut buf);
        let mut builder = DeltaBuilder::with_buffer_size(w, 32);

        builder.add_byte(1)?;
        builder.add_byte(2)?;
        builder.add_byte(3)?;
        builder.add_byte(4)?;
        builder.flush()?;

        assert_eq!(buf, [Op::LiteralN1 as u8, 4, 1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn add_byte_buffer_full1() -> Result<(), Error> {
        let mut buf = Vec::new();
        let w = Cursor::new(&mut buf);
        let mut builder = DeltaBuilder::with_buffer_size(w, 2);

        builder.add_byte(1)?;
        builder.add_byte(2)?;
        builder.add_byte(3)?;
        builder.add_byte(4)?;
        builder.flush()?;

        assert_eq!(
            buf,
            [Op::LiteralN1 as u8, 2, 1, 2, Op::LiteralN1 as u8, 2, 3, 4]
        );
        Ok(())
    }

    #[test]
    fn add_byte_buffer_full2() -> Result<(), Error> {
        let mut buf = Vec::new();
        let w = Cursor::new(&mut buf);
        let mut builder = DeltaBuilder::with_buffer_size(w, 4);

        builder.add_bytes(&[1, 2, 3, 4])?;
        builder.add_byte(5)?;
        builder.flush()?;

        assert_eq!(
            buf,
            [
                Op::LiteralN1 as u8,
                4,
                1,
                2,
                3,
                4,
                Op::LiteralN1 as u8,
                1,
                5
            ]
        );
        Ok(())
    }

    #[test]
    fn add_bytes_buffer_full() -> Result<(), Error> {
        let mut buf = Vec::new();
        let w = Cursor::new(&mut buf);
        let mut builder = DeltaBuilder::with_buffer_size(w, 2);

        builder.add_bytes(&[1, 2, 3])?;
        builder.add_bytes(&[4, 5, 6])?;
        builder.add_bytes(&[7])?;
        builder.flush()?;

        assert_eq!(
            buf,
            [
                Op::LiteralN1 as u8,
                2,
                1,
                2,
                Op::LiteralN1 as u8,
                2,
                3,
                4,
                Op::LiteralN1 as u8,
                2,
                5,
                6,
                Op::LiteralN1 as u8,
                1,
                7
            ]
        );
        Ok(())
    }
}
