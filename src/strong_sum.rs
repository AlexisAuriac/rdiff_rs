use blake2::{digest::consts::U32, Blake2b, Digest};
use md4::Md4;

use crate::error::Error;

pub type Blake2b256 = Blake2b<U32>;

pub const BLAKE2_SUM_LENGTH: u32 = 32;
pub const MD4_SUM_LENGTH: u32 = 16;

pub const MAX_STRONG_SUM_SIZE: usize = 32;

pub type StrongSumBlock = [u8; MAX_STRONG_SUM_SIZE];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrongType {
    Blake2B,
    // md4 is deprecated <https://github.com/librsync/librsync/issues/5>
    Md4,
}

impl StrongType {
    pub fn try_from_str(s: &str) -> Result<Self, Error> {
        match s {
            "blake2" => Ok(StrongType::Blake2B),
            "md4" => Ok(StrongType::Md4),
            _ => Err(Error::BadHashName(s.to_string())),
        }
    }

    pub fn sum_length(&self) -> u32 {
        match self {
            Self::Blake2B => BLAKE2_SUM_LENGTH,
            Self::Md4 => MD4_SUM_LENGTH,
        }
    }
}

#[derive(Debug)]
pub enum StrongSum {
    Blake2b(Blake2b256),
    Md4(Md4),
}

impl StrongSum {
    #[inline]
    pub fn from_type(ss_type: StrongType) -> Self {
        match ss_type {
            StrongType::Blake2B => Self::Blake2b(Blake2b256::new()),
            StrongType::Md4 => Self::Md4(Md4::new()),
        }
    }

    #[inline]
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Self::Blake2b(hasher) => hasher.update(data),
            Self::Md4(hasher) => hasher.update(data),
        }
    }

    #[inline]
    pub fn finalize_reset(&mut self, strong_len: u32) -> Vec<u8> {
        match self {
            Self::Blake2b(hasher) => hasher.finalize_reset()[..(strong_len as usize)].to_vec(),
            Self::Md4(hasher) => hasher.finalize_reset()[..(strong_len as usize)].to_vec(),
        }
    }
}
