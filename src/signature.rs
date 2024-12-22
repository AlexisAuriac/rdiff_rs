use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{self, Read, Write},
    path::Path,
};

use crate::{
    error::Error,
    signature_type::SignatureType,
    strong_sum::{StrongSum, StrongType},
    weak_sum::{WeakSum, WeakSumType},
};

#[derive(Debug, Clone)]
pub struct SignatureOptions {
    block_len: u32,
    strong_len: u32,
    strong_type: StrongType,
    weak_type: WeakSumType,
}

impl SignatureOptions {
    pub fn new() -> Self {
        Self {
            block_len: 2048,
            strong_len: 32,
            strong_type: StrongType::Blake2B,
            weak_type: WeakSumType::RabinKarp,
        }
    }

    pub fn block_len(mut self, block_len: u32) -> Self {
        self.block_len = block_len;
        self
    }

    pub fn strong_len(mut self, strong_len: u32) -> Self {
        self.strong_len = strong_len;
        self
    }

    pub fn strong_type(mut self, strong_type: StrongType) -> Self {
        self.strong_type = strong_type;
        self
    }

    pub fn weak_type(mut self, weak_type: WeakSumType) -> Self {
        self.weak_type = weak_type;
        self
    }

    pub fn signature<I, O>(self, input: &mut I, output: &mut O) -> Result<(), Error>
    where
        I: Read,
        O: Write,
    {
        if self.strong_len > self.strong_type.sum_length() {
            return Err(Error::BadStrongLen(self.strong_len));
        }

        let sigtype = SignatureType::new(self.weak_type, self.strong_type);

        let mut weak = WeakSum::from_type(sigtype.weak_type());
        let mut strong = StrongSum::from_type(sigtype.strong_type());

        output.write_all(&(sigtype as u32).to_be_bytes())?;
        output.write_all(&self.block_len.to_be_bytes())?;
        output.write_all(&self.strong_len.to_be_bytes())?;

        let mut block = vec![0u8; self.block_len as usize];

        loop {
            let n = input.read(&mut block[..])?;
            if n == 0 {
                break;
            }

            let data = &block[..n];

            weak.update(data);
            output.write_all(&weak.digest().to_be_bytes())?;
            weak.reset();

            strong.update(data);
            let strong_sum = strong.finalize_reset(self.strong_len);
            output.write_all(&strong_sum)?;
        }

        output.flush()?;

        Ok(())
    }
}

impl Default for SignatureOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub fn signature<I, O>(input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    SignatureOptions::new().signature(input, output)
}

pub struct Signature {
    pub sigtype: SignatureType,
    pub block_len: u32,
    pub strong_len: u32,
    pub strong_sigs: Vec<Vec<u8>>,
    pub weak2block: HashMap<u32, i32>,
}

pub struct Signature2 {
    pub sigtype: SignatureType,
    pub block_len: u32,
    pub strong_len: u32,
    pub strong_sigs: StrongSums,
    pub weak2block: HashMap<u32, i32>,
}

pub struct StrongSums {
    strong_len: u32,
    data: Vec<u8>,
}

impl StrongSums {
    pub fn new(strong_len: u32) -> Self {
        Self::with_capacity(strong_len, 0)
    }

    pub fn with_capacity(strong_len: u32, nb_sums: usize) -> Self {
        Self {
            strong_len,
            data: vec![0u8; strong_len as usize * nb_sums],
        }
    }

    pub fn push(&mut self, sum: &[u8]) {
        debug_assert!(
            sum.len() == self.strong_len as usize,
            "add sum with wrong size"
        );

        self.data.extend_from_slice(sum);
    }

    pub fn at(&self, block_idx: usize) -> &[u8] {
        let start = block_idx * self.strong_len as usize;
        let end = start + self.strong_len as usize;

        &self.data[start..end]
    }

    pub fn sum_as_mut(&mut self, block_idx: usize) -> &mut [u8] {
        let start = block_idx * self.strong_len as usize;
        let end = start + self.strong_len as usize;

        if end > self.data.len() {
            self.data
                .resize(self.data.len() + self.strong_len as usize, 0);
        }

        &mut self.data[start..end]
    }
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

    input.read_exact(&mut buf32)?;
    let strong_len = u32::from_be_bytes(buf32);

    let input_size = size.unwrap_or(0);
    let nb_blocks = if input_size < 12 {
        // the input size is just wrong
        0
    } else {
        (input_size - 12) / (strong_len as usize + 4)
    };

    let mut strong_sigs: Vec<Vec<u8>> = Vec::with_capacity(nb_blocks);
    let mut weak2block: HashMap<u32, i32> = HashMap::with_capacity(nb_blocks);

    loop {
        let n = input.read(&mut buf32)?;
        if n == 0 {
            break;
        } else if n < 4 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
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

pub fn read_signature_file<P: AsRef<Path>>(path: P) -> Result<Signature, Error> {
    let mut f = OpenOptions::new().read(true).open(path)?;
    let input_size = f.metadata()?.len() as usize;

    read_signature(&mut f, Some(input_size))
    // read_signature(&mut f, None)
}

pub fn read_signature2<I>(input: &mut I, size: Option<usize>) -> Result<Signature2, Error>
where
    I: Read,
{
    let mut buf32 = [0u8; 4];
    input.read_exact(&mut buf32)?;
    let sigtype = u32::from_be_bytes(buf32);
    let sigtype = SignatureType::from_u32(sigtype)?;

    input.read_exact(&mut buf32)?;
    let block_len = u32::from_be_bytes(buf32);

    input.read_exact(&mut buf32)?;
    let strong_len = u32::from_be_bytes(buf32);

    let input_size = size.unwrap_or(0);
    let nb_blocks = if input_size < 12 {
        // the input size is just wrong
        0
    } else {
        (input_size - 12) / (strong_len as usize + 4)
    };

    dbg!(nb_blocks);
    // let mut strong_sigs: Vec<Vec<u8>> = Vec::with_capacity(nb_blocks);
    let mut strong_sigs = StrongSums::with_capacity(strong_len, nb_blocks);
    let mut weak2block: HashMap<u32, i32> = HashMap::with_capacity(nb_blocks);

    let mut i = 0;
    loop {
        let n = input.read(&mut buf32)?;
        if n == 0 {
            break;
        } else if n < 4 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
        }

        let weak_sum = u32::from_be_bytes(buf32);

        input.read_exact(strong_sigs.sum_as_mut(i))?;
        i += 1;

        weak2block.insert(weak_sum, i as i32);
    }

    Ok(Signature2 {
        sigtype,
        block_len,
        strong_len,
        strong_sigs,
        weak2block,
    })
}

pub fn read_signature_file2<P: AsRef<Path>>(path: P) -> Result<Signature2, Error> {
    let mut f = OpenOptions::new().read(true).open(path)?;
    let input_size = f.metadata()?.len() as usize;

    read_signature2(&mut f, Some(input_size))
    // read_signature2(&mut f, None)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::PathBuf};

    use crate::{strong_sum::StrongType, weak_sum::WeakSumType};

    use super::*;

    fn generic_test_signature(
        name: &str,
        weak: &str,
        strong: &str,
        block_len: u32,
        strong_len: u32,
    ) -> Result<(), Error> {
        let file_base_name = format!("{}-{}-{}-{}", name, strong, block_len, strong_len);
        let weak_type = WeakSumType::try_from_str(weak)?;
        let strong_type = StrongType::try_from_str(strong)?;

        let old_data_path = PathBuf::from("testdata").join(name).with_extension("old");
        let mut input = Cursor::new(fs::read(&old_data_path)?);

        let mut output = vec![];

        SignatureOptions::new()
            .block_len(block_len)
            .strong_len(strong_len)
            .weak_type(weak_type)
            .strong_type(strong_type)
            .signature(&mut input, &mut Cursor::new(&mut output))?;
        let sig_size = output.len();
        let got_sig = read_signature(&mut Cursor::new(&output), Some(sig_size))?;

        let want_sig_path = PathBuf::from("testdata")
            .join(file_base_name)
            .with_extension("signature");
        let want_sig_data = fs::read(want_sig_path)?;
        let want_sig = read_signature(&mut Cursor::new(&want_sig_data), Some(want_sig_data.len()))?;

        assert_eq!(got_sig.sigtype, want_sig.sigtype);
        assert_eq!(got_sig.block_len, want_sig.block_len);
        assert_eq!(got_sig.strong_len, want_sig.strong_len);

        assert_eq!(output, want_sig_data);

        Ok(())
    }

    macro_rules! test_signature {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() -> Result<(), Error> {
                    let (name, weak, strong, block_len, strong_len) = $value;
                    generic_test_signature(name, weak, strong, block_len, strong_len)
                }
            )*
        };
    }

    test_signature!(
        signature_000_rollsum_blake2_11_23: ("000", "rollsum", "blake2", 11, 23),
        signature_000_rollsum_blake2_512_32: ("000", "rollsum", "blake2", 512, 32),
        signature_000_rollsum_md4_256_7: ("000", "rollsum", "md4", 256, 7),
        signature_001_rollsum_blake2_512_32: ("001", "rollsum", "blake2", 512, 32),
        signature_001_rollsum_blake2_776_31: ("001", "rollsum", "blake2", 776, 31),
        signature_001_rollsum_md4_777_15: ("001", "rollsum", "md4", 777, 15),
        signature_002_rollsum_blake2_512_32: ("002", "rollsum", "blake2", 512, 32),
        signature_002_rollsum_blake2_431_19: ("002", "rollsum", "blake2", 431, 19),
        signature_002_rollsum_md4_128_16: ("002", "rollsum", "md4", 128, 16),
        signature_003_rollsum_blake2_512_32: ("003", "rollsum", "blake2", 512, 32),
        signature_003_rollsum_blake2_1024_13: ("003", "rollsum", "blake2", 1024, 13),
        signature_003_rollsum_md4_1024_13: ("003", "rollsum", "md4", 1024, 13),
        signature_004_rollsum_blake2_1024_28: ("004", "rollsum", "blake2", 1024, 28),
        signature_004_rollsum_blake2_2222_31: ("004", "rollsum", "blake2", 2222, 31),
        signature_004_rollsum_blake2_512_32: ("004", "rollsum", "blake2", 512, 32),
        signature_005_rollsum_blake2_512_32: ("005", "rollsum", "blake2", 512, 32),
        signature_005_rollsum_blake2_1000_18: ("005", "rollsum", "blake2", 1000, 18),
        signature_005_rollsum_md4_999_14: ("005", "rollsum", "md4", 999, 14),
        signature_006_rollsum_blake2_2_32: ("006", "rollsum", "blake2", 2, 32),
        signature_007_rollsum_blake2_5_32: ("007", "rollsum", "blake2", 5, 32),
        signature_007_rollsum_blake2_4_32: ("007", "rollsum", "blake2", 4, 32),
        signature_007_rollsum_blake2_3_32: ("007", "rollsum", "blake2", 3, 32),
        signature_008_rollsum_blake2_222_30: ("008", "rollsum", "blake2", 222, 30),
        signature_008_rollsum_blake2_512_32: ("008", "rollsum", "blake2", 512, 32),
        signature_008_rollsum_md4_111_11: ("008", "rollsum", "md4", 111, 11),
        signature_009_rollsum_blake2_2048_26: ("009", "rollsum", "blake2", 2048, 26),
        signature_009_rollsum_blake2_512_32: ("009", "rollsum", "blake2", 512, 32),
        signature_009_rollsum_md4_2033_15: ("009", "rollsum", "md4", 2033, 15),
        signature_010_rollsum_blake2_512_32: ("010", "rollsum", "blake2", 512, 32),
        signature_010_rollsum_blake2_7_6: ("010", "rollsum", "blake2", 7, 6),
        signature_010_rollsum_md4_4096_8: ("010", "rollsum", "md4", 4096, 8),
        signature_011_rollsum_blake2_3_32: ("011", "rollsum", "blake2", 3, 32),
        signature_011_rollsum_md4_3_9: ("011", "rollsum", "md4", 3, 9),
    );
}
