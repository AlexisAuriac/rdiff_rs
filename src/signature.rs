use std::collections::{hash_map, HashMap};

use anyhow::{anyhow, Error};
use blake2::{digest::consts::U32, Blake2b, Digest};

use crate::rollsum::Rollsum;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SigType {
    Blake2B = 0x72730137,
    // todo: md4
}

type Blake2b256 = Blake2b<U32>;

impl SigType {
    pub fn from_u32(x: u32) -> Result<Self, Error> {
        match x {
            _ if x == SigType::Blake2B as u32 => Ok(SigType::Blake2B),
            _ => Err(anyhow!("invalid signature type magic")),
        }
    }

    pub fn to_bytes(self) -> [u8; 4] {
        (self as u32).to_be_bytes()
    }

    pub fn strong_sum(self, data: &[u8], strong_len: u32) -> Vec<u8> {
        match self {
            Self::Blake2B => {
                let mut hasher = Blake2b256::new();
                hasher.update(data);
                hasher
                    .finalize()
                    .to_vec()
                    .drain(..(strong_len as usize))
                    .collect()
            }
        }
    }
}

fn compute_weak_checksum(data: &[u8]) -> u32 {
    let mut sum = Rollsum::new();
    sum.update(data);
    return sum.digest();
}

pub fn signature<I, O>(input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: std::io::Read,
    O: std::io::Write,
{
    const block_len: u32 = 2048;
    const strong_len: u32 = 32;
    const sigtype: SigType = SigType::Blake2B;

    // todo: check max strong len

    output.write(&sigtype.to_bytes())?;
    output.write(&block_len.to_be_bytes())?;
    output.write(&strong_len.to_be_bytes())?;

    let mut block = [0u8; block_len as usize];

    loop {
        let n = input.read(&mut block[..])?;
        if n == 0 {
            break;
        }

        let data = &block[..n];

        let weak = compute_weak_checksum(data);
        output.write(&weak.to_be_bytes())?;

        let strong = sigtype.strong_sum(data, strong_len);
        output.write(&strong)?;
    }

    Ok(())
}

pub struct Signature {
    pub sigtype: SigType,
    pub block_len: u32,
    pub strong_len: u32,
    pub strong_sigs: Vec<Vec<u8>>,
    pub weak2block: HashMap<u32, i32>,
}

pub fn read_signature<I>(input: &mut I) -> Result<Signature, Error>
where
    I: std::io::Read,
{
    let mut buf32 = [0u8; 4];
    input.read_exact(&mut buf32)?;
    let sigtype = u32::from_be_bytes(buf32);
    let sigtype = SigType::from_u32(sigtype)?;

    input.read_exact(&mut buf32)?;
    let block_len = u32::from_be_bytes(buf32);

    input.read_exact(&mut buf32)?;
    let strong_len = u32::from_be_bytes(buf32);

    let mut strong_sigs: Vec<Vec<u8>> = vec![];
    let mut weak2block: HashMap<u32, i32> = HashMap::new();

    loop {
        let n = input.read(&mut buf32)?;
        if n == 0 {
            break;
        }
        let weak_sum = u32::from_be_bytes(buf32);

        let mut strong_sum = vec![0u8; strong_len as usize];
        input.read_exact(&mut strong_sum)?;

        weak2block.insert(weak_sum, strong_sigs.len() as i32);
        strong_sigs.push(strong_sum);
    }

    Ok(Signature {
        sigtype,
        block_len,
        strong_len,
        strong_sigs,
        weak2block,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use base64::prelude::*;

    use super::*;

    #[test]
    fn test_signature() -> Result<(), Error> {
        let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
        let sig_b64 = b"cnMBNwAACAAAAAAghvva69bZA09h4vetpuWMJS4VaEyN9/Cxl6ldgPQsoKNoXeJu";

        let mut data_cursor = Cursor::new(data);

        let expect_sig = BASE64_STANDARD.decode(&sig_b64)?;
        let mut out = Cursor::new(Vec::new());

        signature(&mut data_cursor, &mut out)?;

        let actual_sig = out.into_inner();
        assert_eq!(actual_sig, expect_sig);

        Ok(())
    }

    #[test]
    fn test_signature_empty_source() -> Result<(), Error> {
        let data = b"";
        let sig_b64 = b"cnMBNwAACAAAAAAg";

        let mut data_cursor = Cursor::new(data);

        let expect_sig = BASE64_STANDARD.decode(&sig_b64)?;
        let mut out = Cursor::new(Vec::new());

        signature(&mut data_cursor, &mut out)?;

        let actual_sig = out.into_inner();
        assert_eq!(actual_sig, expect_sig);

        Ok(())
    }
}
