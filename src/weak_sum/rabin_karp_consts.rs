pub const RABINKARP_SEED: u32 = 1;
pub const RABINKARP_MULT: u32 = 0x08104225;
pub const RABINKARP_INVM: u32 = 0x98f009ad;
pub const RABINKARP_ADJ: u32 = 0x08104224;

const fn compute_mult_pow2<const N: usize>() -> [u32; N] {
    let mut data = [0; N];
    let mut m = RABINKARP_MULT;

    let mut i = 0;
    while i < N {
        data[i] = m;
        m = m.wrapping_pow(2);
        i += 1;
    }

    data
}

pub static RABINKARP_MULT_POW2: [u32; 32] = compute_mult_pow2();

const fn compute_mult_pow<const N: usize>() -> [u32; N] {
    let mut data = [0; N];
    let mut m: u32 = 1;

    let mut i = 0;
    while i < N {
        data[i] = m;
        m = m.wrapping_mul(RABINKARP_MULT);
        i += 1;
    }

    data
}

// we could go over 2048, but that feels overkill
// goes from RABINKARP_MULT^0 to RABINKARP_MULT^2047 (included)
pub static RABINKARP_MULT_POW: [u32; 2048] = compute_mult_pow();
pub const RABINKARP_MULT_POW_N: u32 = RABINKARP_MULT.wrapping_pow(RABINKARP_MULT_POW.len() as u32);

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn compute_mult_pow2_32() {
//         let expect: [u32; 32] = [
//             0x08104225, 0xa5b71959, 0xf9c080f1, 0x7c71e2e1, 0x0bb409c1, 0x4dc72381, 0xd17a8701,
//             0x96260e01, 0x55101c01, 0x2d303801, 0x66a07001, 0xfe40e001, 0xc081c001, 0x91038001,
//             0x62070001, 0xc40e0001, 0x881c0001, 0x10380001, 0x20700001, 0x40e00001, 0x81c00001,
//             0x03800001, 0x07000001, 0x0e000001, 0x1c000001, 0x38000001, 0x70000001, 0xe0000001,
//             0xc0000001, 0x80000001, 0x00000001, 0x00000001,
//         ];
//         let got = compute_mult_pow2();

//         assert_eq!(expect, got);
//     }

//     #[test]
//     fn compute_mult_pow_32() {
//         let expect: [u32; 32] = [
//             0x08104225, 0xa5b71959, 0x858f9bdd, 0xf9c080f1, 0x5120c4d5, 0x21cb5cc9, 0x64e03b0d,
//             0x7c71e2e1, 0x8f03cc85, 0x9696d939, 0x035e173d, 0x1a6715d1, 0x49960935, 0x8c5efea9,
//             0xf9f2606d, 0x0bb409c1, 0xbf992ae5, 0x04823d19, 0xd423469d, 0x131daeb1, 0xdd63e195,
//             0x80e80489, 0x0343f9cd, 0x0409f4a1, 0x7891dd45, 0x0470c4f9, 0xcea4a9fd, 0xd96fcb91,
//             0x80b3cdf5, 0x7c65ee69, 0x70c2872d, 0x4dc72381,
//         ];
//         let got = compute_mult_pow();

//         assert_eq!(expect, got);
//     }
// }
