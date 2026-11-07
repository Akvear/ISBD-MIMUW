//! Kodowanie kolumny VARCHAR.
//!
//! Układ zakodowanej kolumny (`count` napisów):
//!
//! ```text
//! [len_0][len_1]...[len_{count-1}]   długości napisów jako varint (vle)
//! [ramka zstd]                       skompresowany bufor `data`
//! ```
//!
//! Przykład: ["ab", "", "xyz"]
//!   offsets = [0, 2, 2, 5], data = b"abxyz"
//!   → [0x02, 0x00, 0x03] ++ zstd(b"abxyz")

use super::vle;
use crate::column::Varchar;
use zstd::DEFAULT_COMPRESSION_LEVEL;

pub(crate) fn encode_varchar(column: &Varchar) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..column.len() {
        let len = column.offsets[i + 1] - column.offsets[i];
        vle::encode_u64(len as u64, &mut out);
    }
    let zstd_data = zstd::encode_all(&column.data[..], DEFAULT_COMPRESSION_LEVEL)
        .expect("zstd encode all nie powiodlo sie");
    out.extend_from_slice(&zstd_data);
    out
}

/// Dekoduje `count` napisów zakodowanych przez [`encode_varchar`].
pub(crate) fn decode_varchar(input: &[u8], count: usize) -> Option<Varchar> {
    if count > input.len() {
        return None;
    }

    let mut offsets = Vec::with_capacity(count + 1);
    offsets.push(0u32);
    let mut end = 0u32;
    let mut pos = 0;
    for _ in 0..count {
        let (len, n) = vle::decode_u64(&input[pos..])?;
        pos += n;
        end = end.checked_add(u32::try_from(len).ok()?)?;
        offsets.push(end);
    }

    let data = zstd::decode_all(&input[pos..]).ok()?;
    if data.len() != end as usize {
        return None;
    }

    Some(Varchar { offsets, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Buduje kolumnę bezpośrednio z pól
    fn varchar(strs: &[&[u8]]) -> Varchar {
        let mut offsets = vec![0u32];
        let mut data = Vec::new();
        for s in strs {
            data.extend_from_slice(s);
            offsets.push(data.len() as u32);
        }
        Varchar { offsets, data }
    }

    fn roundtrip(strs: &[&[u8]]) {
        let column = varchar(strs);
        let encoded = encode_varchar(&column);
        let decoded = decode_varchar(&encoded, strs.len()).expect("decode failed");
        assert_eq!(decoded, column);
    }

    #[test]
    fn encodes_lengths_then_zstd() {
        let encoded = encode_varchar(&varchar(&[b"ab", b"", b"xyz"]));
        assert_eq!(&encoded[..3], [0x02, 0x00, 0x03]);
        assert_eq!(zstd::decode_all(&encoded[3..]).unwrap(), b"abxyz");
    }

    #[test]
    fn roundtrip_simple() {
        roundtrip(&[b"ab", b"", b"xyz"]);
        roundtrip(&[b"klakier", b"gargamel"]);
    }

    #[test]
    fn roundtrip_edge_cases() {
        roundtrip(&[]);
        roundtrip(&[b""]);
        roundtrip(&[b"", b"", b""]);
        roundtrip(&[&[b'a'; 300], b"b"]);
        roundtrip(&["zażółć gęślą jaźń".as_bytes()]);
        // dużo powtarzalnych napisów
        let many: Vec<&[u8]> = (0..10_000).map(|_| b"smerf".as_slice()).collect();
        roundtrip(&many);
    }

    #[test]
    fn rejects_invalid() {
        let encoded = encode_varchar(&varchar(&[b"ab", b"xyz"]));
        // count niezgodny z danymi
        assert!(decode_varchar(&encoded, 1).is_none());
        assert!(decode_varchar(&encoded, 3).is_none());
        assert!(decode_varchar(&[], usize::MAX).is_none());
        assert!(decode_varchar(&encoded[..encoded.len() - 1], 2).is_none());
        let mut bad = encoded.clone();
        bad[0] = 3;
        assert!(decode_varchar(&bad, 2).is_none());
        assert!(decode_varchar(&[0x80], 1).is_none());
        let mut huge = Vec::new();
        vle::encode_u64(u32::MAX as u64 + 1, &mut huge);
        huge.extend(zstd::encode_all(&b""[..], 3).unwrap());
        assert!(decode_varchar(&huge, 1).is_none());
    }
}
