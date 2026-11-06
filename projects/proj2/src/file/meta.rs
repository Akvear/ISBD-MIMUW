//! Metadane pliku (footer): schemat i położenie chunków.
//!
//! Format footera (wszystkie liczby jako varint `vle`):
//!
//! ```text
//! num_columns
//!   name_len, name (UTF-8), type (1 bajt: 0 = Int64, 1 = Bool, 2 = Varchar)
//!   ... (num_columns razy)
//! num_row_groups
//!   num_rows
//!   offset, len                 dla każdej kolumny
//!   ... (num_row_groups razy)
//! ```

use crate::encoding::vle;
use crate::error::{KlakierError, Result};
use crate::schema::{DataType, Schema, TableColumn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkMeta {
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowGroupMeta {
    pub num_rows: u64,
    pub chunks: Vec<ChunkMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMeta {
    pub schema: Schema,
    pub row_groups: Vec<RowGroupMeta>,
}

fn put(v: u64, out: &mut Vec<u8>) {
    vle::encode_u64(v, out);
}

impl FileMeta {
    pub fn num_rows(&self) -> u64 {
        self.row_groups.iter().map(|rg| rg.num_rows).sum()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        put(self.schema.table_columns.len() as u64, &mut out);
        for column in &self.schema.table_columns {
            put(column.name.len() as u64, &mut out);
            out.extend_from_slice(column.name.as_bytes());
            out.push(type_to_tag(column.data_type));
        }

        put(self.row_groups.len() as u64, &mut out);
        for rg in &self.row_groups {
            put(rg.num_rows, &mut out);
            for chunk in &rg.chunks {
                put(chunk.offset, &mut out);
                put(chunk.len, &mut out);
            }
        }
        out
    }

    pub fn decode(input: &[u8]) -> Result<Self> {
        let mut r = ByteReader { input, pos: 0 };

        let num_columns = r.u64()?;
        let mut table_columns = Vec::new();
        for _ in 0..num_columns {
            let name_len = r.usize()?;
            let name = std::str::from_utf8(r.bytes(name_len)?)
                .map_err(|_| KlakierError::Corrupted("nazwa kolumny nie jest UTF-8"))?
                .to_owned();
            let data_type = tag_to_type(r.u8()?)?;
            table_columns.push(TableColumn { name, data_type });
        }

        let num_row_groups = r.u64()?;
        let mut row_groups = Vec::new();
        for _ in 0..num_row_groups {
            let num_rows = r.u64()?;
            let mut chunks = Vec::with_capacity(table_columns.len());
            for _ in 0..table_columns.len() {
                let offset = r.u64()?;
                let len = r.u64()?;
                chunks.push(ChunkMeta { offset, len });
            }
            row_groups.push(RowGroupMeta { num_rows, chunks });
        }

        if r.pos != input.len() {
            return Err(KlakierError::Corrupted("nadmiarowe bajty w footerze"));
        }

        Ok(FileMeta {
            schema: Schema::new(table_columns),
            row_groups,
        })
    }
}

fn type_to_tag(data_type: DataType) -> u8 {
    match data_type {
        DataType::Int64 => 0,
        DataType::Bool => 1,
        DataType::Varchar => 2,
    }
}

fn tag_to_type(tag: u8) -> Result<DataType> {
    match tag {
        0 => Ok(DataType::Int64),
        1 => Ok(DataType::Bool),
        2 => Ok(DataType::Varchar),
        _ => Err(KlakierError::Corrupted("nieznany typ kolumny")),
    }
}

struct ByteReader<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn u64(&mut self) -> Result<u64> {
        let (value, n) = vle::decode_u64(&self.input[self.pos..]).ok_or(
            KlakierError::Corrupted("ucięty lub niepoprawny varint w footerze"),
        )?;
        self.pos += n;
        Ok(value)
    }

    fn usize(&mut self) -> Result<usize> {
        usize::try_from(self.u64()?)
            .map_err(|_| KlakierError::Corrupted("wartość poza zakresem usize"))
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&end| end <= self.input.len())
            .ok_or(KlakierError::Corrupted("ucięty footer"))?;
        let slice = &self.input[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FileMeta {
        FileMeta {
            schema: Schema::new(vec![
                TableColumn::new("id", DataType::Int64),
                TableColumn::new("aktywny", DataType::Bool),
                TableColumn::new("imię", DataType::Varchar),
            ]),
            row_groups: vec![
                RowGroupMeta {
                    num_rows: 1024,
                    chunks: vec![
                        ChunkMeta {
                            offset: 4,
                            len: 1100,
                        },
                        ChunkMeta {
                            offset: 1104,
                            len: 128,
                        },
                        ChunkMeta {
                            offset: 1232,
                            len: 5000,
                        },
                    ],
                },
                RowGroupMeta {
                    num_rows: 7,
                    chunks: vec![
                        ChunkMeta {
                            offset: 6232,
                            len: 7,
                        },
                        ChunkMeta {
                            offset: 6239,
                            len: 1,
                        },
                        ChunkMeta {
                            offset: 6240,
                            len: 40,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn roundtrip() {
        let meta = sample();
        assert_eq!(FileMeta::decode(&meta.encode()).unwrap(), meta);
        assert_eq!(meta.num_rows(), 1031);
    }

    #[test]
    fn roundtrip_empty() {
        let meta = FileMeta {
            schema: Schema::new(vec![]),
            row_groups: vec![],
        };
        assert_eq!(FileMeta::decode(&meta.encode()).unwrap(), meta);
    }

    #[test]
    fn exact_bytes() {
        let meta = FileMeta {
            schema: Schema::new(vec![TableColumn::new("a", DataType::Bool)]),
            row_groups: vec![RowGroupMeta {
                num_rows: 3,
                chunks: vec![ChunkMeta { offset: 4, len: 1 }],
            }],
        };
        #[rustfmt::skip]
        let expected = [
            1,          // num_columns
            1, b'a', 1, // "a": Bool
            1,          // num_row_groups
            3,          // num_rows
            4, 1,       // offset, len
        ];
        assert_eq!(meta.encode(), expected);
    }

    fn assert_corrupted(bytes: &[u8]) {
        assert!(
            matches!(FileMeta::decode(bytes), Err(KlakierError::Corrupted(_))),
            "oczekiwano Corrupted dla {bytes:?}"
        );
    }

    #[test]
    fn rejects_invalid() {
        let encoded = sample().encode();
        // każde ucięcie poprawnego footera musi zostać wykryte
        for end in 0..encoded.len() {
            assert_corrupted(&encoded[..end]);
        }
        // nadmiarowy bajt
        let mut extra = encoded.clone();
        extra.push(0);
        assert_corrupted(&extra);
        // nieznany typ kolumny
        assert_corrupted(&[1, 1, b'a', 9, 0]);
        // absurdalna liczba kolumn daje błąd zamiast próby alokacji
        let mut huge = vec![1];
        vle::encode_u64(u64::MAX, &mut huge);
        assert_corrupted(&huge);
    }
}
