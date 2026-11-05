//! Kodowanie kolumn do bajtów i z powrotem.

pub mod boolean;
pub mod varchar;
pub mod vle;
pub mod zigzag;

pub use boolean::{decode_bool, encode_bool};
pub use varchar::{decode_varchar, encode_varchar};

/// Koduje kolumnę int64: delta → zigzag → varint.
pub fn encode_int64(values: &[i64]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut prev = 0i64;
    for &v in values {
        let delta = v.wrapping_sub(prev);
        prev = v;
        vle::encode_u64(zigzag::encode(delta), &mut out);
    }
    out
}

/// Dekoduje `count` wartości zakodowanych przez [`encode_int64`].
pub fn decode_int64(input: &[u8], count: usize) -> Option<Vec<i64>> {
    if count > input.len() {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    let mut pos = 0;
    let mut prev = 0i64;
    for _ in 0..count {
        let (raw, n) = vle::decode_u64(&input[pos..])?;
        pos += n;
        prev = prev.wrapping_add(zigzag::decode(raw));
        out.push(prev);
    }
    if pos != input.len() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_int64() {
        let values = [-1000, -500, 0, 500, 1000, i64::MAX, i64::MIN];
        let encoded = encode_int64(&values);
        let decoded = decode_int64(&encoded, values.len()).expect("Decoding failed");
        for (original, decoded) in values.iter().zip(decoded.iter()) {
            assert_eq!(original, decoded);
        }
    }
}
