use std::sync::Arc;

use crate::column::Column;
use crate::schema::Schema;

pub const MAX_BATCH_SIZE: usize = 1024;

/// Dlaczego nie udało się zbudować batcha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchError {
    /// Liczba kolumn różna od liczby pól w schemacie.
    ColumnCountMismatch { expected: usize, actual: usize },
    /// Typ kolumny `index` niezgodny ze schematem.
    TypeMismatch { index: usize },
    /// Kolumna `index` ma inną liczbę wierszy niż kolumna 0.
    LengthMismatch {
        index: usize,
        expected: usize,
        actual: usize,
    },
    /// Liczba wierszy przekracza [`MAX_BATCH_SIZE`].
    TooManyRows(usize),
}

/// Batch danych: schemat + kolumny o równej długości.
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    schema: Arc<Schema>,
    columns: Vec<Column>,
    num_rows: usize,
}

impl Batch {
    /// Buduje batch, sprawdzając, że:
    /// - kolumn jest tyle, ile pól w schemacie,
    /// - typ każdej kolumny zgadza się ze schematem (`Column::data_type`),
    /// - wszystkie kolumny mają tę samą długość,
    /// - liczba wierszy ≤ MAX_BATCH_SIZE.
    pub fn try_new(schema: Arc<Schema>, columns: Vec<Column>) -> Result<Self, BatchError> {
        if schema.table_columns.len() != columns.len() {
            return Err(BatchError::ColumnCountMismatch {
                expected: schema.table_columns.len(),
                actual: columns.len(),
            });
        }

        for (i, (col, field)) in columns.iter().zip(schema.table_columns.iter()).enumerate() {
            if col.data_type() != field.data_type {
                return Err(BatchError::TypeMismatch { index: i });
            }
        }

        if columns.is_empty() {
            return Ok(Self {
                schema,
                columns,
                num_rows: 0,
            });
        }

        if columns[0].len() > MAX_BATCH_SIZE {
            return Err(BatchError::TooManyRows(columns[0].len()));
        }

        let num_rows = columns[0].len();
        for (i, col) in columns.iter().enumerate() {
            if col.len() != num_rows {
                return Err(BatchError::LengthMismatch {
                    index: i,
                    expected: num_rows,
                    actual: col.len(),
                });
            }
        }

        Ok(Self {
            schema,
            columns,
            num_rows,
        })
    }

    /// Kolumna o indeksie `i`. Panikuje, jeśli `i` poza zakresem.
    pub fn column(&self, i: usize) -> &Column {
        self.columns
            .get(i)
            .expect("Batch: indeks kolumny poza zakresem")
    }

    /// Kolumna o podanej nazwie.
    pub fn column_by_name(&self, name: &str) -> Option<&Column> {
        self.columns.get(self.schema.index_of(name)?)
    }

    pub fn schema(&self) -> &Arc<Schema> {
        &self.schema
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
}

// TODO: testy
#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Column;
    use crate::column::Varchar;
    use crate::schema::DataType;
    use crate::schema::TableColumn;

    #[test]
    fn test_batch_try_new() {
        let schema = Arc::new(Schema::new(vec![
            TableColumn::new("a", DataType::Int64),
            TableColumn::new("b", DataType::Bool),
            TableColumn::new("c", DataType::Varchar),
            TableColumn::new("d", DataType::Varchar),
        ]));

        let batch = Batch::try_new(
            schema.clone(),
            vec![
                Column::Int64(vec![1, 2, 3]),
                Column::Bool(vec![true, false, true]),
                Column::Varchar(Varchar::from_iter(["abc", "def", "ghi"])),
                Column::Varchar(Varchar::from_iter(["jkl", "mnopqrst", "uvwxyz"])),
            ],
        );
        assert!(batch.is_ok());
    }

    #[test]
    fn batch_size_limit() {
        let schema = Arc::new(Schema::new(vec![TableColumn::new("a", DataType::Int64)]));
        let col = Column::Int64(vec![0; MAX_BATCH_SIZE]);
        let batch = Batch::try_new(schema.clone(), vec![col.clone()]);
        assert!(batch.is_ok());

        let col_too_big = Column::Int64(vec![0; MAX_BATCH_SIZE + 1]);
        let res = Batch::try_new(schema, vec![col_too_big]);
        assert!(matches!(
            res,
            Err(BatchError::TooManyRows(n)) if n == MAX_BATCH_SIZE + 1
        ));
    }
}
