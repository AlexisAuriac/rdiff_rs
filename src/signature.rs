mod read_signature;

pub use read_signature::*;

use std::io::{Read, Write};

use crate::{
    error::Error,
    signature_type::SignatureType,
    strong_sum::{StrongSum, StrongType},
    weak_sum::{WeakSum, WeakSumType},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrongLenOption {
    Min,
    Max,
    N(u32), // more than zero
}

#[derive(Debug, Clone)]
pub struct SignatureOptions {
    block_len: Option<u32>, // more than zero
    strong_len: StrongLenOption,
    strong_type: StrongType,
    weak_type: WeakSumType,
    input_size: Option<usize>,
}

pub const DEFAULT_BLOCK_LEN: u32 = 2048;
pub const DEFAULT_MIN_STRONG_LEN: u32 = 12;

pub const MIN_BLOCK_LEN: u32 = 256;
pub const INPUT_SIZE_FOR_MIN_BLOCK_LEN: usize = 65536;

fn recommend_block_len(size: usize) -> u32 {
    if size <= INPUT_SIZE_FOR_MIN_BLOCK_LEN {
        MIN_BLOCK_LEN
    } else {
        (size as f64).sqrt().floor() as u32 & !127
    }
}

fn recommend_min_strong_len(size: usize, block_len: u32) -> u32 {
    2 + ((size + (1 << 24)).ilog2() + ((size as u32 / block_len + 1).ilog2() + 7)) / 8
}

impl SignatureOptions {
    pub fn new() -> Self {
        Self {
            block_len: None,
            strong_len: StrongLenOption::Max,
            strong_type: StrongType::Blake2B,
            weak_type: WeakSumType::RabinKarp,
            input_size: None,
        }
    }

    pub fn block_len(&mut self, block_len: u32) -> &mut Self {
        // the original librsync uses 0 to mean use recommended value
        // also, we just don't want block_len to be 0
        if block_len > 0 {
            self.block_len = Some(block_len);
        }

        self
    }

    pub fn min_strong_len(&mut self) -> &mut Self {
        self.strong_len = StrongLenOption::Min;
        self
    }

    pub fn max_strong_len(&mut self) -> &mut Self {
        self.strong_len = StrongLenOption::Max;
        self
    }

    pub fn strong_len(&mut self, strong_len: u32) -> &mut Self {
        // the original librsync uses 0 to mean max and -1 to mean min
        // also, we just don't want strong_len to be 0
        if strong_len == 0 {
            self.strong_len = StrongLenOption::Max;
        } else {
            self.strong_len = StrongLenOption::N(strong_len);
        }

        self
    }

    pub fn strong_type(&mut self, strong_type: StrongType) -> &mut Self {
        self.strong_type = strong_type;
        self
    }

    pub fn weak_type(&mut self, weak_type: WeakSumType) -> &mut Self {
        self.weak_type = weak_type;
        self
    }

    pub fn input_size(&mut self, size: usize) -> &mut Self {
        self.input_size = Some(size);
        self
    }

    fn recommended_block_len(&self) -> u32 {
        if let Some(block_len) = self.block_len {
            return block_len;
        }

        let size = match self.input_size {
            None | Some(0) => return DEFAULT_BLOCK_LEN,
            Some(size) => size,
        };

        recommend_block_len(size)
    }

    fn recommended_strong_len(&self, block_len: u32) -> Result<u32, Error> {
        if let StrongLenOption::N(strong_len) = self.strong_len {
            debug_assert!(strong_len > 0, "strong len must be more than 0");

            return if strong_len > self.strong_type.sum_length() {
                Err(Error::BadStrongLen(strong_len))
            } else {
                Ok(strong_len)
            };
        }

        match (self.strong_len, self.input_size) {
            (StrongLenOption::Min, Some(size)) => Ok(recommend_min_strong_len(size, block_len)),
            (StrongLenOption::Min, None) => Ok(DEFAULT_MIN_STRONG_LEN),
            (StrongLenOption::Max, _) => Ok(self.strong_type.sum_length()),
            (StrongLenOption::N(_), _) => unreachable!(),
        }
    }

    pub fn signature<I, O>(&self, input: &mut I, output: &mut O) -> Result<(), Error>
    where
        I: Read,
        O: Write,
    {
        let block_len = self.recommended_block_len();
        debug_assert!(block_len > 0, "block len must be more than 0");

        let strong_len = self.recommended_strong_len(block_len)?;
        debug_assert!(strong_len > 0, "strong len must be more than 0");

        let sigtype = SignatureType::new(self.weak_type, self.strong_type);

        let mut weak = WeakSum::from_type(sigtype.weak_type());
        let mut strong = StrongSum::from_type(sigtype.strong_type());

        output.write_all(&(sigtype as u32).to_be_bytes())?;
        output.write_all(&block_len.to_be_bytes())?;
        output.write_all(&strong_len.to_be_bytes())?;

        let mut block = vec![0u8; block_len as usize];

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
            let strong_sum = strong.finalize_reset(strong_len);
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

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::PathBuf};

    use crate::signature::read_signature;

    use super::*;

    #[test]
    fn test_recommend_block_len() {
        assert_eq!(recommend_block_len(0), MIN_BLOCK_LEN);
        assert_eq!(recommend_block_len(1000000), 896);
    }

    #[test]
    fn test_recommend_min_strong_len() {
        assert_eq!(recommend_min_strong_len(0, 256), 5);
        assert_eq!(recommend_min_strong_len(1000000, 896), 7);
    }

    fn generic_test_signature(
        name: &str,
        weak: &str,
        strong: &str,
        block_len: u32,
        strong_len: u32,
    ) -> Result<(), Error> {
        let file_base_name = format!("{}-{}-{}-{}", name, strong, block_len, strong_len);
        let weak_type = weak.parse()?;
        let strong_type = strong.parse()?;

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
