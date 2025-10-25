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
    // todo: check block_len > 0

    input.read_exact(&mut buf32)?;
    let strong_len = u32::from_be_bytes(buf32);
    // todo: check strong_len makes sense

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
