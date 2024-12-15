use std::io::{Read, Write};

use crate::{
    delta_match::{Match, MatchKind},
    error::Error,
    ring_buffer::RingBuffer,
    rollsum::Rollsum,
    signature::Signature,
};

const OUTPUT_BUFFER_SIZE: usize = 16 * 1024 * 1024;

pub const DELTA_MAGIC: u32 = 0x72730236;

fn delta_buf<I, O>(
    sig: &Signature,
    input: &mut I,
    output: &mut O,
    lit_buff: Vec<u8>,
) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    debug_assert!(
        lit_buff.is_empty() && lit_buff.capacity() == OUTPUT_BUFFER_SIZE,
        "bad literal buffer"
    );

    output.write_all(&DELTA_MAGIC.to_be_bytes())?;

    let mut m = Match::new(output, lit_buff);

    let mut weaksum = Rollsum::new();
    let mut ring_buf = RingBuffer::new(sig.block_len as usize);

    let mut buf = vec![0u8; sig.block_len as usize];

    loop {
        let read_count = if weaksum.count() < sig.block_len as usize {
            sig.block_len as usize - weaksum.count()
        } else {
            1
        };

        let n = input.read(&mut buf[..read_count])?;
        if n == 0 {
            break;
        }
        let data = &buf[..n];

        weaksum.update(data);

        if weaksum.count() < sig.block_len as usize {
            ring_buf.write(data);
            continue;
        }

        if weaksum.count() > sig.block_len as usize {
            let prev_byte = ring_buf.front().unwrap_or(0);

            m.add(MatchKind::Literal, prev_byte as u64, 1)?;
            weaksum.roll_out(prev_byte);
        }

        ring_buf.write(data);

        let digest = weaksum.digest();
        if let Some(block_idx) = sig.weak2block.get(&digest) {
            let strong2 = sig.sigtype.strong_sum(ring_buf.as_bytes(), sig.strong_len);
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
    delta_buf(sig, input, output, buf)?;

    Ok(())
}
