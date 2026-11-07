//! ZigZag: mapuje liczby ze znakiem na bez znaku tak, żeby małe co do
//! modułu wartości dawały małe (w sensie ilosci potrzebnych bitów) wyniki.
//!
//!  0 → 0,  -1 → 1,  1 → 2,  -2 → 3,  2 → 4, ...

pub(crate) fn encode(value: i64) -> u64 {
    ((value >> 63) ^ (value << 1)) as u64
}

pub(crate) fn decode(value: u64) -> i64 {
    (value >> 1) as i64 ^ -((value & 1) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        for i in -1000..=1000 {
            let encoded = encode(i);
            let decoded = decode(encoded);
            assert_eq!(i, decoded, "Failed for i = {}", i);
        }
    }

    #[test]
    fn test_encode_decode_edge_cases() {
        let edge_cases = [i64::MIN, i64::MIN + 1, 0, i64::MAX - 1, i64::MAX];
        for &i in &edge_cases {
            let encoded = encode(i);
            let decoded = decode(encoded);
            assert_eq!(i, decoded, "Failed for edge case i = {}", i);
        }
    }
}
