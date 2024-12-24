pub mod builder;

use std::io::{Read, Write};

use builder::DeltaBuilder;

use crate::{
    error::Error, ring_buffer::RingBuffer, signature::Signature, strong_sum::StrongSum,
    weak_sum::WeakSum,
};

pub fn delta<I, O>(sig: &Signature, input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    let mut builder = DeltaBuilder::new(output);
    builder.write_magic()?;

    let mut weak = WeakSum::from_type(sig.sigtype.weak_type());
    let mut strong = StrongSum::from_type(sig.sigtype.strong_type());

    let mut ring_buf = RingBuffer::new(sig.block_len as usize);

    let mut buf = vec![0u8; sig.block_len as usize];

    loop {
        let read_count = if weak.count() < sig.block_len as usize {
            sig.block_len as usize - weak.count()
        } else {
            1
        };

        let n = input.read(&mut buf[..read_count])?;
        if n == 0 {
            break;
        }
        let data = &buf[..n];

        if n == 1 {
            weak.rollin(data[0]);
        } else {
            weak.update(data);
        }

        if weak.count() < sig.block_len as usize {
            ring_buf.write(data);
            continue;
        }

        if weak.count() > sig.block_len as usize {
            let prev_byte = ring_buf.front().unwrap_or(0);

            builder.add_byte(prev_byte)?;
            weak.rollout(prev_byte);
        }

        ring_buf.write(data);

        let digest = weak.digest();
        if let Some(block_idx) = sig.weak2block.get(&digest) {
            strong.update(ring_buf.as_bytes());
            let strong_sum = strong.finalize_reset(sig.strong_len);

            if sig.strong_sigs[*block_idx as usize] == strong_sum[..] {
                weak.reset();
                ring_buf.reset();

                builder.copy(
                    *block_idx as u64 * sig.block_len as u64,
                    sig.block_len as u64,
                )?;
            }
        }
    }

    builder.add_bytes(ring_buf.as_bytes())?;
    builder.flush()?;
    builder.end()?;

    Ok(())
}
