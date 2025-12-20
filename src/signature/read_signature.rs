use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{self, Read},
    path::Path,
};

use crate::{
    error::Error,
    signature_type::SignatureType,
    strong_sum::{MAX_STRONG_SUM_SIZE, StrongSumBlock},
};

pub struct Signature {
    pub sigtype: SignatureType,
    pub block_len: u32,
    pub strong_len: u32,
    pub strong_sigs: Vec<StrongSumBlock>,
    pub weak2block: HashMap<u32, i32>,
}

pub fn read_signature<I>(input: &mut I, size: Option<usize>) -> Result<Signature, Error>
where
    I: Read,
{
    let mut buf32 = [0u8; 4];
    input.read_exact(&mut buf32)?;
    let sigtype = u32::from_be_bytes(buf32);
    let sigtype = SignatureType::from_u32(sigtype)?;

    input.read_exact(&mut buf32)?;
    let block_len = u32::from_be_bytes(buf32);
    if block_len == 0 {
        return Err(Error::ZeroBlockLen);
    }

    input.read_exact(&mut buf32)?;
    let strong_len = u32::from_be_bytes(buf32);
    if strong_len == 0 || strong_len > sigtype.strong_type().sum_length() {
        return Err(Error::BadStrongLen(strong_len));
    }

    let input_size = size.unwrap_or(0);
    let nb_blocks = if input_size < 12 {
        // the input size is just wrong
        0
    } else {
        (input_size - 12) / (strong_len as usize + 4)
    };

    let mut strong_sigs = Vec::with_capacity(nb_blocks);
    let mut weak2block = HashMap::with_capacity(nb_blocks);

    loop {
        let n = input.read(&mut buf32)?;
        if n == 0 {
            break;
        } else if n < 4 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
        }
        let weak_sum = u32::from_be_bytes(buf32);

        let mut strong_sum = [0u8; MAX_STRONG_SUM_SIZE];
        let buf_strong_sum = &mut strong_sum[..strong_len as usize];
        input.read_exact(buf_strong_sum)?;

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

pub fn read_signature_file<P>(path: P) -> Result<Signature, Error>
where
    P: AsRef<Path>,
{
    let mut f = OpenOptions::new().read(true).open(path)?;
    let input_size = f.metadata()?.len() as usize;

    read_signature(&mut f, Some(input_size))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn make_bad_signature(sigtype: u32, block_len: u32, strong_len: u32, data: &[u8]) -> Vec<u8> {
        let mut signature = Vec::with_capacity(12 + data.len());

        signature.extend(sigtype.to_be_bytes());
        signature.extend(block_len.to_be_bytes());
        signature.extend(strong_len.to_be_bytes());
        signature.extend(data);

        signature
    }

    #[test]
    fn test_bad_sigtype() {
        let bad_sigtype = 0x01234567;
        let sig = make_bad_signature(bad_sigtype, 2048, 32, &[]);

        match read_signature(&mut Cursor::new(sig), None) {
            Ok(_) => panic!("expected error on bad sigtype"),
            Err(Error::BadSigType(sigtype)) if sigtype == bad_sigtype => (),
            Err(e) => panic!(
                "expected error {:?}, got {e:?}",
                Error::BadSigType(bad_sigtype)
            ),
        }
    }

    #[test]
    fn test_zero_block_len() {
        let sig = make_bad_signature(SignatureType::RkBlake2B as u32, 0, 32, &[]);

        match read_signature(&mut Cursor::new(sig), None) {
            Ok(_) => panic!("expected error on block_len == 0"),
            Err(Error::ZeroBlockLen) => (),
            Err(e) => panic!("expected error {:?}, got {e:?}", Error::ZeroBlockLen),
        }
    }

    #[test]
    fn test_zero_strong_len() {
        let sig = make_bad_signature(SignatureType::RkBlake2B as u32, 2048, 0, &[]);

        match read_signature(&mut Cursor::new(sig), None) {
            Ok(_) => panic!("expected error on strong_len == 0"),
            Err(Error::BadStrongLen(0)) => (),
            Err(e) => panic!("expected error {:?}, got {e:?}", Error::BadStrongLen(0)),
        }
    }

    #[test]
    fn test_strong_len_too_large_md4_1() {
        let sig = make_bad_signature(SignatureType::Md4 as u32, 2048, 100, &[]);

        match read_signature(&mut Cursor::new(sig), None) {
            Ok(_) => panic!("expected error on strong_len too large"),
            Err(Error::BadStrongLen(100)) => (),
            Err(e) => panic!("expected {:?}, got {e:?}", Error::BadStrongLen(100)),
        }
    }

    #[test]
    fn test_strong_len_too_large_md4_2() {
        let sig = make_bad_signature(SignatureType::Md4 as u32, 2048, 24, &[]);

        match read_signature(&mut Cursor::new(sig), None) {
            Ok(_) => panic!("expected error on strong_len too large"),
            Err(Error::BadStrongLen(24)) => (),
            Err(e) => panic!("expected {:?}, got {e:?}", Error::BadStrongLen(24)),
        }
    }

    #[test]
    fn test_strong_len_too_large_blake2b() {
        let sig = make_bad_signature(SignatureType::Blake2B as u32, 2048, 33, &[]);

        match read_signature(&mut Cursor::new(sig), None) {
            Ok(_) => panic!("expected error on strong_len too large"),
            Err(Error::BadStrongLen(33)) => (),
            Err(e) => panic!("expected {:?}, got {e:?}", Error::BadStrongLen(33)),
        }
    }

    #[test]
    fn test_bad_input_size_too_small() {
        let sig = make_bad_signature(SignatureType::RkBlake2B as u32, 2048, 32, &[]);

        match read_signature(&mut Cursor::new(sig), Some(5)) {
            Ok(_) => (),
            Err(e) => panic!("inaccurate input size should be quietly ignored, got {e:?}"),
        }
    }

    #[test]
    fn test_bad_input_size_too_large() {
        let sig = make_bad_signature(SignatureType::RkBlake2B as u32, 2048, 32, &[]);

        match read_signature(&mut Cursor::new(sig), Some(4096)) {
            Ok(_) => (),
            Err(e) => panic!("inaccurate input size should be quietly ignored, got {e:?}"),
        }
    }
}
