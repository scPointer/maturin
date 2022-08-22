#![no_std]
#![deny(warnings, unsafe_code, unstable_features)]

// §3
pub mod binary;
// §4
pub mod base;
// §5
pub mod legacy;
// §6
pub mod time;
// §7
pub mod spi;
// §8
pub mod rfnc;
// §9
pub mod hsm;
// §10
pub mod srst;
// §11
pub mod pmu;

/// Converts SBI EID from str.
const fn eid_from_str(name: &str) -> i32 {
    match *name.as_bytes() {
        [a] => i32::from_be_bytes([0, 0, 0, a]),
        [a, b] => i32::from_be_bytes([0, 0, a, b]),
        [a, b, c] => i32::from_be_bytes([0, a, b, c]),
        [a, b, c, d] => i32::from_be_bytes([a, b, c, d]),
        _ => unreachable!(),
    }
}
