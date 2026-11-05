//! Kodowanie kolumny z bool dokładnie 1 bit na wartość
//!
//! Przyklad: `[true, false, true, true]` → `[00001101]`.

pub fn encode_bool(values: &[bool]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len().div_ceil(8));
    for chunk in values.chunks(8) {
        let mut byte = 0u8;
        for (bit, &v) in chunk.iter().enumerate() {
            byte |= (v as u8) << bit;
        }
        out.push(byte);
    }
    out
}

/// Dekoduje `count` wartości zakodowanych przez [`encode_bool`].
pub fn decode_bool(input: &[u8], count: usize) -> Option<Vec<bool>> {
    if input.len() != count.div_ceil(8) {
        return None;
    }
    // Czy w ostatnim bajcie nie ma ustawionych bitów poza `count`.
    let tail = count % 8;
    if tail != 0 && input[input.len() - 1] >> tail != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() * 8);
    for &byte in input {
        for bit in 0..8 {
            out.push((byte >> bit) & 1 == 1);
        }
    }
    out.truncate(count);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_lsb_first() {
        assert_eq!(encode_bool(&[true, false, true, true]), [0x0D]);
        let mut nine = [false; 9];
        nine[0] = true;
        assert_eq!(encode_bool(&nine), [0x01, 0x00]);
    }

    #[test]
    fn roundtrip() {
        for n in [0usize, 1, 7, 8, 9, 1000] {
            let values: Vec<bool> = (0..n).map(|i| i % 3 == 0).collect();
            let encoded = encode_bool(&values);
            assert_eq!(encoded.len(), n.div_ceil(8));
            assert_eq!(decode_bool(&encoded, n).unwrap(), values);
        }
    }

    #[test]
    fn rejects_invalid() {
        // za mało / za dużo bajtów względem count
        assert!(decode_bool(&[0x00], 9).is_none());
        assert!(decode_bool(&[0x00, 0x00], 8).is_none());
        // niezerowy padding: count = 4, a bit 4 ustawiony
        assert!(decode_bool(&[0x10], 4).is_none());
    }
}
