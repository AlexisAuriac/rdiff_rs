use std::io::{Read, Write};

use anyhow::{anyhow, Error};

use crate::{
    delta_match::{Match, MatchKind},
    ring_buffer::RingBuffer,
    rollsum::Rollsum,
    signature::Signature,
};

const OUTPUT_BUFFER_SIZE: usize = 16 * 1024 * 1024;

pub const DELTA_MAGIC: u32 = 0x72730236;

pub fn delta_buf<I, O>(
    sig: &Signature,
    input: &mut I,
    output: &mut O,
    lit_buff: Vec<u8>,
) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    if lit_buff.len() != 0 || lit_buff.capacity() != OUTPUT_BUFFER_SIZE {
        return Err(anyhow!("bad literal buffer"));
    }

    output.write(&DELTA_MAGIC.to_be_bytes())?;

    let mut prev_byte = 0u8;
    let mut m = Match::new(output, lit_buff);

    let mut weaksum = Rollsum::new();
    let mut ring_buf = RingBuffer::new(sig.block_len as usize);

    loop {
        let mut buf1 = [0u8];

        let n = input.read(&mut buf1[..])?;
        if n == 0 {
            break;
        }
        let b = buf1[0];

        if ring_buf.total_written() > 0 {
            prev_byte = ring_buf.get(0).unwrap();
        }
        ring_buf.write_byte(b);
        weaksum.roll_in(b);

        if weaksum.count() < sig.block_len as usize {
            continue;
        } else if weaksum.count() > sig.block_len as usize {
            m.add(MatchKind::Literal, prev_byte as u64, 1)?;
            weaksum.roll_out(prev_byte);
        }

        let digest = weaksum.digest();
        if let Some(block_idx) = sig.weak2block.get(&digest) {
            let strong2 = sig.sigtype.strong_sum(&ring_buf.as_bytes(), sig.strong_len);
            if sig.strong_sigs[*block_idx as usize] == strong2 {
                weaksum.reset();
                ring_buf.reset();
                m.add(
                    MatchKind::Copy,
                    *block_idx as u64 * sig.block_len as u64,
                    sig.block_len as u64,
                )?;
            }
        }
    }

    for b in ring_buf.as_bytes() {
        m.add(MatchKind::Literal, *b as u64, 1)?;
    }

    m.flush()?;
    m.end()?;

    Ok(())
}

pub fn delta<I, O>(sig: &Signature, input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    let buf = Vec::with_capacity(OUTPUT_BUFFER_SIZE);
    return delta_buf(sig, input, output, buf);
}
