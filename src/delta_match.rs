use std::io::Write;

use anyhow::Error;

use crate::op::Op;

const OUTPUT_BUFFER_SIZE: u64 = 16 * 1024 * 1024;

fn int_size(d: u64) -> u8 {
    if d >= 2u64.pow(32) {
        8
    } else if d >= 2u64.pow(16) {
        4
    } else if d >= 2u64.pow(8) {
        2
    } else {
        1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    Literal,
    Copy,
}

pub struct Match<O> {
    kind: MatchKind,
    pos: u64,
    len: u64,
    output: O,
    lit: Vec<u8>,
}

impl<O: Write> Match<O> {
    pub fn new(output: O, buff: Vec<u8>) -> Self {
        Self {
            output,
            lit: buff,
            kind: MatchKind::Literal,
            pos: 0,
            len: 0,
        }
    }

    pub fn write(&mut self, d: u64, size: u8) -> Result<(), Error> {
        match size {
            1 => self.output.write_all(&(d as u8).to_be_bytes())?,
            2 => self.output.write_all(&(d as u16).to_be_bytes())?,
            4 => self.output.write_all(&(d as u32).to_be_bytes())?,
            8 => self.output.write_all(&d.to_be_bytes())?,
            _ => unimplemented!(), // todo: fuck this
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        if self.len == 0 {
            return Ok(());
        }

        let pos_size = int_size(self.pos);
        let len_size = int_size(self.len);

        match self.kind {
            MatchKind::Copy => {
                let cmd = match (pos_size, len_size) {
                    (1, 1) => Op::CopyN1N1,
                    (1, 2) => Op::CopyN1N2,
                    (1, 4) => Op::CopyN1N4,
                    (1, 8) => Op::CopyN1N8,
                    (2, 1) => Op::CopyN2N1,
                    (2, 2) => Op::CopyN2N2,
                    (2, 4) => Op::CopyN2N4,
                    (2, 8) => Op::CopyN2N8,
                    (4, 1) => Op::CopyN4N1,
                    (4, 2) => Op::CopyN4N2,
                    (4, 4) => Op::CopyN4N4,
                    (4, 8) => Op::CopyN4N8,
                    (8, 1) => Op::CopyN8N1,
                    (8, 2) => Op::CopyN8N2,
                    (8, 4) => Op::CopyN8N4,
                    (8, 8) => Op::CopyN8N8,
                    _ => unimplemented!(),
                };

                self.output.write_all(&(cmd as u8).to_be_bytes())?;
                self.write(self.pos, pos_size)?;
                self.write(self.len, len_size)?;
            }
            MatchKind::Literal => {
                let cmd = match len_size {
                    1 => Op::LiteralN1,
                    2 => Op::LiteralN2,
                    4 => Op::LiteralN4,
                    8 => Op::LiteralN8,
                    _ => unimplemented!(),
                };

                self.output.write_all(&(cmd as u8).to_be_bytes())?;
                self.write(self.len, len_size)?;
                self.output.write_all(&self.lit)?;
                self.lit.truncate(0);
            }
        }

        self.pos = 0;
        self.len = 0;

        Ok(())
    }

    pub fn add(&mut self, kind: MatchKind, pos: u64, len: u64) -> Result<(), Error> {
        if len != 0 && self.kind != kind {
            self.flush()?;
        }

        self.kind = kind;

        match kind {
            MatchKind::Literal => {
                self.lit.push(pos as u8);
                self.len += 1;

                if self.len >= OUTPUT_BUFFER_SIZE {
                    self.flush()?
                }
            }
            MatchKind::Copy => {
                if self.pos + self.len != pos {
                    self.flush()?;
                    self.pos = pos;
                    self.len = len;
                } else {
                    self.len += len;
                }
            }
        }

        Ok(())
    }

    pub fn end(mut self) -> Result<(), Error> {
        self.output.write_all(&(Op::EndOp as u8).to_be_bytes())?;
        Ok(())
    }
}
