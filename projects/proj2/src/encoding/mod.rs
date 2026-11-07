//! Kodowanie kolumn do bajtów i z powrotem.

pub(crate) mod boolean;
pub(crate) mod varchar;
pub(crate) mod vle;
pub(crate) mod zigzag;

pub(crate) use boolean::{decode_bool, encode_bool};
pub(crate) use varchar::{decode_varchar, encode_varchar};

use crate::column::Column;
use crate::schema::DataType;

pub(crate) fn encode_column(column: &Column) -> Vec<u8> {
    match column {
        Column::Int64(values) => encode_int64(values),
        Column::Bool(values) => encode_bool(values),
        Column::Varchar(values) => encode_varchar(values),
    }
}

pub(crate) fn decode_column(input: &[u8], data_type: DataType, count: usize) -> Option<Column> {
    Some(match data_type {
        DataType::Int64 => Column::Int64(decode_int64(input, count)?),
        DataType::Bool => Column::Bool(decode_bool(input, count)?),
        DataType::Varchar => Column::Varchar(decode_varchar(input, count)?),
    })
}

/// Koduje kolumnę int64: delta → zigzag → varint.
pub(crate) fn encode_int64(values: &[i64]) -> Vec<u8> {
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
pub(crate) fn decode_int64(input: &[u8], count: usize) -> Option<Vec<i64>> {
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

    #[test]
    fn column_roundtrip_all_types() {
        use crate::column::Varchar;
        let columns = [
            Column::Int64(vec![3, -1, i64::MAX]),
            Column::Bool(vec![true, false, true]),
            Column::Varchar(Varchar::from_iter(["ab", "", "klakier"])),
        ];
        for column in columns {
            let encoded = encode_column(&column);
            let decoded = decode_column(&encoded, column.data_type(), column.len());
            assert_eq!(decoded, Some(column));
        }
    }

    #[test]
    fn decode_column_wrong_type_or_count() {
        let encoded = encode_column(&Column::Bool(vec![true; 3]));
        // ten sam chunk zdekodowany z innym count lub jako inny typ
        assert_eq!(decode_column(&encoded, DataType::Bool, 9), None);
        assert_eq!(decode_column(&encoded, DataType::Varchar, 1), None);
    }
}
