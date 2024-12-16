use crate::{error::Error, strong_sum::StrongType, weak_sum::weak_sum::WeakSumType};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureType {
    RkBlake2B = 0x72730147, // default
    Blake2B = 0x72730137,
    // md4 is deprecated <https://github.com/librsync/librsync/issues/5>
    Md4 = 0x72730136,
    RkMd4 = 0x72730146,
}

impl SignatureType {
    pub fn from_u32(x: u32) -> Result<Self, Error> {
        match x {
            _ if x == SignatureType::RkBlake2B as u32 => Ok(SignatureType::RkBlake2B),
            _ if x == SignatureType::Blake2B as u32 => Ok(SignatureType::Blake2B),
            _ if x == SignatureType::RkMd4 as u32 => Ok(SignatureType::RkMd4),
            _ if x == SignatureType::Md4 as u32 => Ok(SignatureType::Md4),
            _ => Err(Error::BadSigType(x)),
        }
    }

    pub fn new(weak: WeakSumType, strong: StrongType) -> Self {
        match (weak, strong) {
            (WeakSumType::RabinKarp, StrongType::Blake2B) => SignatureType::RkBlake2B,
            (WeakSumType::Rollsum, StrongType::Blake2B) => SignatureType::Blake2B,
            (WeakSumType::RabinKarp, StrongType::Md4) => SignatureType::RkMd4,
            (WeakSumType::Rollsum, StrongType::Md4) => SignatureType::Md4,
        }
    }

    pub fn weak_type(&self) -> WeakSumType {
        match self {
            SignatureType::RkBlake2B | SignatureType::RkMd4 => WeakSumType::RabinKarp,
            SignatureType::Blake2B | SignatureType::Md4 => WeakSumType::Rollsum,
        }
    }

    pub fn strong_type(&self) -> StrongType {
        match self {
            SignatureType::RkBlake2B | SignatureType::Blake2B => StrongType::Blake2B,
            SignatureType::RkMd4 | SignatureType::Md4 => StrongType::Md4,
        }
    }
}
