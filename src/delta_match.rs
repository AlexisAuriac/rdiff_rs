use std::io::Write;

use anyhow::Error;

use crate::op::{Op, OpArgLen};

const OUTPUT_BUFFER_SIZE: u64 = 16 * 1024 * 1024;

fn int_size(d: u64) -> OpArgLen {
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

    pub fn write(&mut self, d: u64, size: OpArgLen) -> Result<(), Error> {
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

        let pos_size = int_size(self.pos);
        let len_size = int_size(self.len);

        match self.kind {
            MatchKind::Copy => {
                let op = match (pos_size, len_size) {
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
                };

                self.output.write_all(&(op as u8).to_be_bytes())?;
                self.write(self.pos, pos_size)?;
                self.write(self.len, len_size)?;
            }
            MatchKind::Literal => {
                let cmd = match len_size {
                    OpArgLen::N1 => Op::LiteralN1,
                    OpArgLen::N2 => Op::LiteralN2,
                    OpArgLen::N4 => Op::LiteralN4,
                    OpArgLen::N8 => Op::LiteralN8,
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
