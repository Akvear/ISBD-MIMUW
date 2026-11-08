//! Zapis pliku klakier: batch po batchu, footer na końcu.

use std::io::Write;
use std::sync::Arc;

use crate::batch::Batch;
use crate::encoding::encode_column;
use crate::error::{KlakierError, Result};
use crate::file::MAGIC;
use crate::file::meta::{ChunkMeta, FileMeta, RowGroupMeta};
use crate::schema::Schema;

pub struct FileWriter<W: Write> {
    inner: W,
    schema: Arc<Schema>,
    pos: u64,
    row_groups: Vec<RowGroupMeta>,
}

impl<W: Write> FileWriter<W> {
    /// Tworzy writer i od razu zapisuje MAGIC na początku pliku.
    pub fn new(inner: W, schema: Arc<Schema>) -> Result<Self> {
        let mut writer = Self {
            inner,
            schema,
            pos: 0,
            row_groups: Vec::new(),
        };
        writer.inner.write_all(MAGIC)?;
        writer.pos = MAGIC.len() as u64;
        Ok(writer)
    }

    /// Zapisuje batch jako jeden row group.
    ///
    /// Zwraca `SchemaMismatch`, jeśli schemat batcha różni się od schematu
    /// pliku.
    pub fn write_batch(&mut self, batch: &Batch) -> Result<()> {
        if *self.schema != **batch.schema() {
            return Err(KlakierError::SchemaMismatch);
        }

        if batch.num_rows() == 0 {
            return Ok(());
        }

        let mut chunks = Vec::new();
        for col in batch.columns() {
            let bytes = encode_column(col);
            self.inner.write_all(&bytes)?;
            chunks.push(ChunkMeta {
                offset: self.pos,
                len: bytes.len() as u64,
            });
            self.pos += bytes.len() as u64;
        }

        self.row_groups.push(RowGroupMeta {
            num_rows: batch.num_rows() as u64,
            chunks,
        });

        Ok(())
    }

    /// Zapisuje footer, footer_len i MAGIC, robi flush i zwraca
    /// wewnętrzny writer.
    pub fn finish(mut self) -> Result<W> {
        let footer = FileMeta {
            schema: (*self.schema).clone(),
            row_groups: self.row_groups,
        };
        let footer_bytes = footer.encode();
        let footer_len = u32::try_from(footer_bytes.len())
            .map_err(|_| KlakierError::Corrupted("footer pliku jest zbyt długi"))?;
        self.inner.write_all(&footer_bytes)?;
        self.inner.write_all(&footer_len.to_le_bytes())?;
        self.inner.write_all(MAGIC)?;
        self.inner.flush()?;
        Ok(self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_writer() {
        let schema = Arc::new(Schema::new(vec![crate::schema::TableColumn::new(
            "a",
            crate::schema::DataType::Bool,
        )]));
        let mut writer = FileWriter::new(Vec::new(), schema.clone()).unwrap();
        let batch = Batch::try_new(
            schema.clone(),
            vec![crate::column::Column::Bool(vec![true, false, true])],
        )
        .unwrap();

        for _ in 0..10 {
            writer.write_batch(&batch).unwrap();
        }

        let output = writer.finish().unwrap();
        let footer_len_bytes = &output[output.len() - 8..output.len() - 4];
        let footer_len = u32::from_le_bytes(footer_len_bytes.try_into().unwrap()) as usize;
        let footer_bytes = &output[output.len() - 8 - footer_len..output.len() - 8];
        let footer = FileMeta::decode(footer_bytes).unwrap();
        assert_eq!(footer.schema, *schema);
        assert_eq!(footer.row_groups.len(), 10);
        for rg in footer.row_groups {
            assert_eq!(rg.num_rows, 3);
            assert_eq!(rg.chunks.len(), 1);
        }
    }
}
