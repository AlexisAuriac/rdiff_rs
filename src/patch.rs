use std::io::{copy, Read, Seek, SeekFrom, Write};

use anyhow::{anyhow, Error};

use crate::{
    delta::DELTA_MAGIC,
    op::{OpKind, OP2CMD},
};

fn read_param<I>(i: &mut I, size: u8) -> Result<i64, Error>
where
    I: Read,
{
    match size {
        1 => {
            let mut buf = [0u8; 1];
            i.read_exact(&mut buf)?;
            Ok(buf[0] as i64)
        }
        2 => {
            let mut buf = [0u8; 2];
            i.read_exact(&mut buf)?;
            Ok(u16::from_be_bytes(buf) as i64)
        }
        4 => {
            let mut buf = [0u8; 4];
            i.read_exact(&mut buf)?;
            Ok(u32::from_be_bytes(buf) as i64)
        }
        8 => {
            let mut buf = [0u8; 8];
            i.read_exact(&mut buf)?;
            Ok(u64::from_be_bytes(buf) as i64)
        }
        _ => Ok(0),
    }
}

pub fn patch<I, D, O>(old: &mut I, delta: &mut D, out: &mut O) -> Result<(), Error>
where
    I: Read + Seek,
    D: Read,
    O: Write,
{
    let mut magic_buf = [0u8; 4];
    delta.read_exact(&mut magic_buf)?;
    let magic = u32::from_be_bytes(magic_buf);

    if magic != DELTA_MAGIC {
        return Err(anyhow!("bad magic"));
    }

    loop {
        let mut op_buf = [0u8];
        delta.read_exact(&mut op_buf)?;
        let op = op_buf[0];
        let cmd = &OP2CMD[op as usize];

        let (param1, param2) = if cmd.len1 == 0 {
            (cmd.immediate as i64, 0)
        } else {
            let param1 = read_param(delta, cmd.len1)?;
            let param2 = read_param(delta, cmd.len2)?;
            (param1, param2)
        };

        match cmd.kind {
            OpKind::Literal => {
                copy(&mut delta.take(param1 as u64), out)?;
            }
            OpKind::Copy => {
                old.seek(SeekFrom::Start(param1 as u64))?;
                copy(&mut old.take(param2 as u64), out)?;
            }
            OpKind::End => break,
            _ => return Err(anyhow!("bogus command {:?}", cmd.kind)),
        }
    }

    Ok(())
}
