//! Odczyt pliku klakier: footer na starcie, potem row groupy na żądanie.

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use crate::batch::{Batch, MAX_BATCH_SIZE};
use crate::encoding::decode_column;
use crate::error::{KlakierError, Result};
use crate::file::meta::FileMeta;
use crate::file::{MAGIC, TRAILER_LEN};
use crate::schema::Schema;

pub struct FileReader<R: Read + Seek> {
    inner: R,
    meta: FileMeta,
    /// Schemat w `Arc`, współdzielony przez wszystkie zwracane batche.
    schema: Arc<Schema>,
    /// Offset początku footera = koniec obszaru danych. Każdy chunk musi
    /// leżeć w przedziale [MAGIC.len(), data_end).
    data_end: u64,
}

impl<R: Read + Seek> FileReader<R> {
    /// Otwiera plik: sprawdza magic na obu końcach i wczytuje footer.
    pub fn open(mut inner: R) -> Result<Self> {
        let file_len = inner.seek(SeekFrom::End(0))?;
        if file_len < MAGIC.len() as u64 + TRAILER_LEN as u64 {
            return Err(KlakierError::Corrupted("plik jest zbyt krótki"));
        }

        inner.seek(SeekFrom::Start(0))?;
        let mut magic = [0; 4];
        inner.read_exact(&mut magic)?;
        if magic != *MAGIC {
            return Err(KlakierError::Corrupted("plik nie ma poprawnego magic"));
        }

        inner.seek(SeekFrom::End(-(TRAILER_LEN as i64)))?;
        let mut trailer = [0; TRAILER_LEN];
        inner.read_exact(&mut trailer)?;
        let footer_len = u32::from_le_bytes(trailer[..4].try_into().unwrap()) as u64;
        if trailer[4..] != *MAGIC {
            return Err(KlakierError::Corrupted(
                "plik nie ma poprawnego magic w stopce",
            ));
        }
        let data_end = file_len
            .checked_sub(TRAILER_LEN as u64)
            .and_then(|x| x.checked_sub(footer_len))
            .ok_or(KlakierError::Corrupted("footer pliku jest zbyt długi"))?;

        let mut footer_bytes = vec![0; footer_len as usize];
        inner.seek(SeekFrom::Start(data_end))?;
        inner.read_exact(&mut footer_bytes)?;
        let meta = FileMeta::decode(&footer_bytes)?;
        let schema = Arc::new(meta.schema.clone());

        Ok(FileReader {
            inner,
            meta,
            schema,
            data_end,
        })
    }

    pub fn schema(&self) -> &Arc<Schema> {
        &self.schema
    }

    pub fn num_rows(&self) -> u64 {
        self.meta.num_rows()
    }

    pub fn num_row_groups(&self) -> usize {
        self.meta.row_groups.len()
    }

    /// Wczytuje i dekoduje row group `i` jako batch.
    pub fn read_row_group(&mut self, i: usize) -> Result<Batch> {
        if i >= self.meta.row_groups.len() {
            return Err(KlakierError::Corrupted("row group index poza zakresem"));
        }

        let rg = &self.meta.row_groups[i];
        if rg.num_rows > MAX_BATCH_SIZE as u64 {
            return Err(KlakierError::Corrupted("row group ma zbyt wiele wierszy"));
        }

        let mut columns = Vec::with_capacity(self.schema.table_columns.len());

        for (chunk, table_column) in rg.chunks.iter().zip(self.schema.table_columns.iter()) {
            if chunk.offset < MAGIC.len() as u64 {
                return Err(KlakierError::Corrupted("chunk zaczyna się przed danymi"));
            }
            if chunk
                .offset
                .checked_add(chunk.len)
                .is_none_or(|end| end > self.data_end)
            {
                return Err(KlakierError::Corrupted("chunk wychodzi poza dane"));
            }

            let bytes = Self::read_at(&mut self.inner, chunk.offset, chunk.len)?;
            let column = decode_column(&bytes, table_column.data_type, rg.num_rows as usize)
                .ok_or(KlakierError::Corrupted("nie można zdekodować kolumny"))?;

            columns.push(column);
        }

        Batch::try_new(self.schema.clone(), columns).map_err(KlakierError::Batch)
    }

    /// Iterator po wszystkich batchach pliku, w kolejności row groupów.
    pub fn batches(&mut self) -> Batches<'_, R> {
        Batches {
            reader: self,
            next: 0,
        }
    }

    /// Czyta `len` bajtów od `offset`.
    fn read_at(inner: &mut R, offset: u64, len: u64) -> Result<Vec<u8>> {
        inner.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0; len as usize];
        inner.read_exact(&mut buf)?;
        Ok(buf)
    }
}

/// Iterator zwracany przez [`FileReader::batches`].
pub struct Batches<'a, R: Read + Seek> {
    reader: &'a mut FileReader<R>,
    next: usize,
}

impl<R: Read + Seek> Iterator for Batches<'_, R> {
    type Item = Result<Batch>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.reader.num_row_groups() {
            let result = self.reader.read_row_group(self.next);
            self.next += 1;
            Some(result)
        } else {
            None
        }
    }
}
