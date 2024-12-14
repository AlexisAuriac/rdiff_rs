use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use anyhow::{anyhow, Error};
use blake2::{digest::consts::U32, Blake2b, Digest};
use md4::Md4;

use crate::rollsum::Rollsum;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigType {
    Blake2B = 0x72730137,
    // deprecated <https://github.com/librsync/librsync/issues/5>
    Md4 = 0x72730136,
}

type Blake2b256 = Blake2b<U32>;

const BLAKE2_SUM_LENGTH: u32 = 32;
const MD4_SUM_LENGTH: u32 = 16;

impl SigType {
    pub fn from_u32(x: u32) -> Result<Self, Error> {
        match x {
            _ if x == SigType::Blake2B as u32 => Ok(SigType::Blake2B),
            _ if x == SigType::Md4 as u32 => Ok(SigType::Md4),
            _ => Err(anyhow!("invalid signature type magic")),
        }
    }

    pub fn try_from_str(s: &str) -> Result<Self, Error> {
        match s {
            "blake2" => Ok(SigType::Blake2B),
            "md4" => Ok(SigType::Md4),
            _ => Err(anyhow!("{}: invalid signature type", s)),
        }
    }

    pub fn sum_length(&self) -> u32 {
        match self {
            Self::Blake2B => BLAKE2_SUM_LENGTH,
            Self::Md4 => MD4_SUM_LENGTH,
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
                hasher.finalize()[..(strong_len as usize)].to_vec()
            }
            Self::Md4 => {
                let mut hasher = Md4::new();
                hasher.update(data);
                hasher.finalize()[..(strong_len as usize)].to_vec()
            }
        }
    }
}

fn compute_weak_checksum(data: &[u8]) -> u32 {
    let mut sum = Rollsum::new();
    sum.update(data);
    sum.digest()
}

pub fn signature<I, O>(
    input: &mut I,
    output: &mut O,
    block_len: u32,
    strong_len: u32,
    sigtype: SigType,
) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    if strong_len > sigtype.sum_length() {
        return Err(anyhow!(
            "invalid strong len {} for sigtype {:?}",
            strong_len,
            sigtype
        ));
    }

    output.write_all(&sigtype.to_bytes())?;
    output.write_all(&block_len.to_be_bytes())?;
    output.write_all(&strong_len.to_be_bytes())?;

    let mut block = vec![0u8; block_len as usize];

    loop {
        let n = input.read(&mut block[..])?;
        if n == 0 {
            break;
        }

        let data = &block[..n];

        let weak = compute_weak_checksum(data);
        output.write_all(&weak.to_be_bytes())?;

        let strong = sigtype.strong_sum(data, strong_len);
        output.write_all(&strong)?;
    }

    output.flush()?;

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
    I: Read,
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
        } else if n < 4 {
            return Err(anyhow!("unexpected EOF while reading weak sum"));
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

pub fn read_signature_file(path: &Path) -> Result<Signature, Error> {
    let mut f = OpenOptions::new().read(true).open(path)?;
    read_signature(&mut f)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::PathBuf};

    use super::*;

    macro_rules! test_signature {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let (name, sigtype, block_len, strong_len) = $value;
                    let file_base_name = format!("{}-{}-{}-{}", name, sigtype, block_len, strong_len);
                    let sigtype = SigType::try_from_str(sigtype)?;

                    let old_data_path = PathBuf::from("testdata").join(name).with_extension("old");
                    let mut input = Cursor::new(fs::read(&old_data_path)?);

                    let mut output = Cursor::new(vec![]);

                    signature(&mut input, &mut output, block_len, strong_len, sigtype)?;
                    output.set_position(0);
                    let got_sig = read_signature(&mut output)?;

                    let want_sig_path = PathBuf::from("testdata")
                        .join(file_base_name)
                        .with_extension("signature");
                    let mut want_sig_data = Cursor::new(fs::read(want_sig_path)?);
                    let want_sig = read_signature(&mut want_sig_data)?;

                    assert_eq!(got_sig.sigtype, want_sig.sigtype);
                    assert_eq!(got_sig.block_len, want_sig.block_len);
                    assert_eq!(got_sig.strong_len, want_sig.strong_len);

                    assert_eq!(output.into_inner(), want_sig_data.into_inner());

                    Ok(())
                }
            )*
        };
    }

    test_signature!(
        signature_000_blake2_11_23: ("000", "blake2", 11, 23),
        signature_000_blake2_512_32: ("000", "blake2", 512, 32),
        signature_000_md4_256_7: ("000", "md4", 256, 7),
        signature_001_blake2_512_32: ("001", "blake2", 512, 32),
        signature_001_blake2_776_31: ("001", "blake2", 776, 31),
        signature_001_md4_777_15: ("001", "md4", 777, 15),
        signature_002_blake2_512_32: ("002", "blake2", 512, 32),
        signature_002_blake2_431_19: ("002", "blake2", 431, 19),
        signature_002_md4_128_16: ("002", "md4", 128, 16),
        signature_003_blake2_512_32: ("003", "blake2", 512, 32),
        signature_003_blake2_1024_13: ("003", "blake2", 1024, 13),
        signature_003_md4_1024_13: ("003", "md4", 1024, 13),
        signature_004_blake2_1024_28: ("004", "blake2", 1024, 28),
        signature_004_blake2_2222_31: ("004", "blake2", 2222, 31),
        signature_004_blake2_512_32: ("004", "blake2", 512, 32),
        signature_005_blake2_512_32: ("005", "blake2", 512, 32),
        signature_005_blake2_1000_18: ("005", "blake2", 1000, 18),
        signature_005_md4_999_14: ("005", "md4", 999, 14),
        signature_006_blake2_2_32: ("006", "blake2", 2, 32),
        signature_007_blake2_5_32: ("007", "blake2", 5, 32),
        signature_007_blake2_4_32: ("007", "blake2", 4, 32),
        signature_007_blake2_3_32: ("007", "blake2", 3, 32),
        signature_008_blake2_222_30: ("008", "blake2", 222, 30),
        signature_008_blake2_512_32: ("008", "blake2", 512, 32),
        signature_008_md4_111_11: ("008", "md4", 111, 11),
        signature_009_blake2_2048_26: ("009", "blake2", 2048, 26),
        signature_009_blake2_512_32: ("009", "blake2", 512, 32),
        signature_009_md4_2033_15: ("009", "md4", 2033, 15),
        signature_010_blake2_512_32: ("010", "blake2", 512, 32),
        signature_010_blake2_7_6: ("010", "blake2", 7, 6),
        signature_010_md4_4096_8: ("010", "md4", 4096, 8),
        signature_011_blake2_3_32: ("011", "blake2", 3, 32),
        signature_011_md4_3_9: ("011", "md4", 3, 9),
    );
}
