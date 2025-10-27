#![feature(portable_simd)]

pub mod buf_reader_with_retry;
pub mod delta;
pub mod error;
pub mod op;
pub mod patch;
pub mod ring_buffer;
pub mod signature;
pub mod signature_type;
pub mod strong_sum;
pub mod weak_sum;
pub mod whole;
