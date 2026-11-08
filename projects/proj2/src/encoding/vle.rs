//! VLE dla `u64`.
//!
//! Każdy bajt niesie 7 bitów wartości (od najmłodszych).
//! Najwyższy bit = 1 oznacza, że po nim jest kolejny bajt.

/// Maksymalna liczba bajtów potrzebna na zakodowanie `u64` (ceil(64 / 7)).
pub(crate) const MAX_LEN: usize = 10;

/// Dopisuje zakodowaną wartość na koniec `out`.
pub(crate) fn encode_u64(value: u64, out: &mut Vec<u8>) {
    let mut value = value;
    loop {
        let byte = (value & 0x7F) as u8;
        if value > 0x7F {
            out.push(byte | 0x80);
        } else {
            out.push(byte);
            break;
        }
        value >>= 7;
    }
}

/// Czyta jedną liczbę z początku `input`.
pub(crate) fn decode_u64(input: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0;
    for (i, &byte) in input.iter().take(MAX_LEN).enumerate() {
        // 10 bajt może nieść tylko 1 bit i nie może mieć kontynuacji
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None
}

// TODO: testy
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let values = [0, 1, 127, 128, 255, 300, u64::MAX];
        for &value in &values {
            let mut encoded = Vec::new();
            encode_u64(value, &mut encoded);
            let (decoded, len) = decode_u64(&encoded).expect("Decoding failed");
            assert_eq!(value, decoded);
            assert_eq!(len, encoded.len());
        }
    }

    #[test]
    fn test_decode_invalid() {
        let invalid_data = [vec![0x80], vec![0xFF; 11]];
        for data in invalid_data {
            assert!(
                decode_u64(&data).is_none(),
                "Invalid data should return None: {:?}",
                data
            );
        }
    }
}
