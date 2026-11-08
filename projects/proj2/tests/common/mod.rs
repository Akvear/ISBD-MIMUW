//! Wspólne pomocnicze funkcje dla testów end-to-end.

#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;

use klakier::Result;
use klakier::batch::{Batch, MAX_BATCH_SIZE};
use klakier::column::{Column, Varchar};
use klakier::file::{FileReader, FileWriter};
use klakier::schema::{DataType, Schema, TableColumn};

/// Schemat jak w przykładowym pliku z polecenia:
/// 1× INT64, 1× BOOLEAN, 2× VARCHAR.
pub fn sample_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        TableColumn::new("id", DataType::Int64),
        TableColumn::new("aktywny", DataType::Bool),
        TableColumn::new("imie", DataType::Varchar),
        TableColumn::new("opis", DataType::Varchar),
    ]))
}

pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(5734587975384793498)
            .wrapping_add(3952005928390528311);
        self.0 >> 11
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

const NAMES: [&str; 6] = [
    "Klakier",
    "Gargamel",
    "Papa Smerf",
    "Smerfetka",
    "Łasuch",
    "",
];

pub fn sample_batch(schema: &Arc<Schema>, first_id: i64, num_rows: usize, seed: u64) -> Batch {
    assert!(num_rows <= MAX_BATCH_SIZE);
    let mut rng = Lcg::new(seed);

    let ids: Vec<i64> = (0..num_rows as i64).map(|i| first_id + i).collect();
    let flags: Vec<bool> = (0..num_rows).map(|_| rng.below(3) == 0).collect();
    let names: Varchar = (0..num_rows)
        .map(|_| NAMES[rng.below(NAMES.len() as u64) as usize])
        .collect();
    let descriptions: Varchar = (0..num_rows)
        .map(|_| {
            let len = rng.below(200) as usize;
            (0..len)
                .map(|_| (b'a' + rng.below(26) as u8) as char)
                .collect::<String>()
        })
        .collect();

    Batch::try_new(
        schema.clone(),
        vec![
            Column::Int64(ids),
            Column::Bool(flags),
            Column::Varchar(names),
            Column::Varchar(descriptions),
        ],
    )
    .expect("sample_batch: niepoprawny batch")
}

pub fn sample_batches(schema: &Arc<Schema>, num_batches: usize, last_rows: usize) -> Vec<Batch> {
    (0..num_batches)
        .map(|i| {
            let rows = if i + 1 == num_batches {
                last_rows
            } else {
                MAX_BATCH_SIZE
            };
            sample_batch(schema, (i * MAX_BATCH_SIZE) as i64, rows, i as u64)
        })
        .collect()
}

pub fn write_file(path: &Path, schema: &Arc<Schema>, batches: &[Batch]) -> Result<()> {
    let file = BufWriter::new(File::create(path)?);
    let mut writer = FileWriter::new(file, schema.clone())?;
    for batch in batches {
        writer.write_batch(batch)?;
    }
    writer.finish()?;
    Ok(())
}

pub fn read_file(path: &Path) -> Result<(Arc<Schema>, Vec<Batch>)> {
    let mut reader = FileReader::open(BufReader::new(File::open(path)?))?;
    let schema = reader.schema().clone();
    let batches = reader.batches().collect::<Result<Vec<_>>>()?;
    Ok((schema, batches))
}
