//! Generuje przykładowy plik klakier z losowymi danymi:
//! 1× INT64, 1× BOOLEAN, 2× VARCHAR.
//!
//! Użycie:
//!     cargo run --bin gen <plik.klk> [liczba_wierszy=10000]

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;

use klakier::batch::{Batch, MAX_BATCH_SIZE};
use klakier::column::{Column, Varchar};
use klakier::file::FileWriter;
use klakier::schema::{DataType, Schema, TableColumn};
use rand::RngExt;
use rand::rngs::ThreadRng;

/// Znaki do losowych napisów — część spoza ASCII, żeby liczba znaków ASCII różniła się od liczby bajtów.
const CHARS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'k', 'l', 'm', 'o', 'r', 's', 't', 'z', ' ', 'ą', 'ę', 'ł', 'ó', 'ż',
];

fn random_string(rng: &mut ThreadRng, max_len: usize) -> String {
    let len = rng.random_range(0..=max_len);
    (0..len)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())])
        .collect()
}

fn random_batch(schema: &Arc<Schema>, num_rows: usize) -> Batch {
    let mut rng = rand::rng();
    let ids = (0..num_rows)
        .map(|_| rng.random_range(-1_000_000..1_000_000))
        .collect();
    let flags = (0..num_rows).map(|_| rng.random_bool(0.3)).collect();
    let names: Varchar = (0..num_rows).map(|_| random_string(&mut rng, 10)).collect();
    let descriptions: Varchar = (0..num_rows)
        .map(|_| random_string(&mut rng, 100))
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
    .expect("wygenerowany batch jest zgodny ze schematem")
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .ok_or("użycie: gen <plik.klk> [liczba_wierszy]")?;
    let num_rows: usize = match args.get(2) {
        Some(n) => n.parse()?,
        None => 10_000,
    };

    let schema = Arc::new(Schema::new(vec![
        TableColumn::new("id", DataType::Int64),
        TableColumn::new("aktywny", DataType::Bool),
        TableColumn::new("imie", DataType::Varchar),
        TableColumn::new("opis", DataType::Varchar),
    ]));

    let mut writer = FileWriter::new(BufWriter::new(File::create(path)?), schema.clone())?;
    let mut remaining = num_rows;
    while remaining > 0 {
        let rows = remaining.min(MAX_BATCH_SIZE);
        writer.write_batch(&random_batch(&schema, rows))?;
        remaining -= rows;
    }
    writer.finish()?;

    println!("Zapisano {path}: {num_rows} wierszy");
    Ok(())
}
