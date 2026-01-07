pub mod rabin_karp;
pub mod rabin_karp_consts;
pub mod rollsum;

use std::{fmt::Display, str::FromStr};

use crate::error::Error;

use rabin_karp::RabinKarp;
use rollsum::Rollsum;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeakSumType {
    Rollsum,
    RabinKarp,
}

impl FromStr for WeakSumType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rabinkarp" => Ok(WeakSumType::RabinKarp),
            "rollsum" => Ok(WeakSumType::Rollsum),
            _ => Err(Error::BadRollsumName(s.to_string())),
        }
    }
}

impl Display for WeakSumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeakSumType::Rollsum => write!(f, "rollsum"),
            WeakSumType::RabinKarp => write!(f, "rabinkarp"),
        }
    }
}

#[derive(Debug)]
pub enum WeakSum {
    RollSum(Rollsum),
    RabinKarp(RabinKarp),
}

impl WeakSum {
    #[inline]
    pub fn from_type(ws_type: WeakSumType) -> Self {
        match ws_type {
            WeakSumType::RabinKarp => Self::RabinKarp(RabinKarp::new()),
            WeakSumType::Rollsum => Self::RollSum(Rollsum::new()),
        }
    }

    #[inline]
    pub fn count(&self) -> usize {
        match self {
            WeakSum::RollSum(sum) => sum.count(),
            WeakSum::RabinKarp(sum) => sum.count(),
        }
    }

    pub fn update(&mut self, p: &[u8]) {
        match self {
            WeakSum::RollSum(sum) => sum.update(p),
            WeakSum::RabinKarp(sum) => sum.update(p),
        }
    }

    #[inline]
    pub fn rotate(&mut self, outb: u8, inb: u8) {
        match self {
            WeakSum::RollSum(sum) => sum.rotate(outb, inb),
            WeakSum::RabinKarp(sum) => sum.rotate(outb, inb),
        }
    }

    #[inline]
    pub fn rollin(&mut self, inb: u8) {
        match self {
            WeakSum::RollSum(sum) => sum.rollin(inb),
            WeakSum::RabinKarp(sum) => sum.rollin(inb),
        }
    }

    #[inline]
    pub fn rollout(&mut self, outb: u8) {
        match self {
            WeakSum::RollSum(sum) => sum.rollout(outb),
            WeakSum::RabinKarp(sum) => sum.rollout(outb),
        }
    }

    #[inline]
    pub fn digest(&self) -> u32 {
        match self {
            WeakSum::RollSum(sum) => sum.digest(),
            WeakSum::RabinKarp(sum) => sum.digest(),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        match self {
            WeakSum::RollSum(sum) => sum.reset(),
            WeakSum::RabinKarp(sum) => sum.reset(),
        }
    }
}
