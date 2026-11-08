//! Wczytuje plik w formacie klakier i liczy metryki.
//! - INT64: średnia,
//! - BOOLEAN: entropia binarna,
//! - VARCHAR: liczba znaków ASCII.
//!
//! Użycie:
//!     cargo run --bin stats <plik.klk>

use klakier::file::FileReader;
use klakier::schema::DataType;
use std::io::BufReader;

fn open_file(path: &str) -> FileReader<BufReader<std::fs::File>> {
    let file = std::fs::File::open(path).expect("nie można otworzyć pliku");
    let reader = BufReader::new(file);
    FileReader::open(reader).expect("nie można wczytać pliku")
}

fn binary_entropy(p: f64) -> f64 {
    if p == 0.0 || p == 1.0 {
        return 0.0;
    }
    -p * p.log2() - (1.0 - p) * (1.0 - p).log2()
}

struct Totals {
    num_rows: u64,
    int_sums: Vec<i128>,
    true_counts: Vec<u64>,
    ascii_counts: Vec<u64>,
}

fn calculate_metrics(reader: &mut FileReader<BufReader<std::fs::File>>) -> Totals {
    let num_columns = reader.schema().table_columns.len();
    let mut totals = Totals {
        num_rows: 0,
        int_sums: vec![0; num_columns],
        true_counts: vec![0; num_columns],
        ascii_counts: vec![0; num_columns],
    };

    for batch in reader.batches() {
        let batch = batch.expect("błąd przy wczytywaniu batcha");
        totals.num_rows += batch.num_rows() as u64;

        for (i, col) in batch.columns().iter().enumerate() {
            if let Some(ints) = col.as_int64() {
                totals.int_sums[i] += ints.iter().map(|&x| x as i128).sum::<i128>();
            }
            if let Some(bools) = col.as_bool() {
                totals.true_counts[i] += bools.iter().filter(|&&b| b).count() as u64;
            }
            if let Some(strings) = col.as_varchar() {
                totals.ascii_counts[i] +=
                    strings.data().iter().filter(|b| b.is_ascii()).count() as u64;
            }
        }
    }

    totals
}

fn main() {
    let path = std::env::args().nth(1).expect("użycie: stats <plik.klk>");
    let mut reader = open_file(&path);
    let totals = calculate_metrics(&mut reader);

    println!("Plik: {path}, wierszy: {}", totals.num_rows);
    for (i, column) in reader.schema().table_columns.iter().enumerate() {
        let name = &column.name;
        match column.data_type {
            DataType::Int64 => {
                if totals.num_rows == 0 {
                    println!("{name} (INT64): średnia = brak (0 wierszy)");
                } else {
                    let average = totals.int_sums[i] as f64 / totals.num_rows as f64;
                    println!("{name} (INT64): średnia = {average}");
                }
            }
            DataType::Bool => {
                let p = if totals.num_rows == 0 {
                    0.0
                } else {
                    totals.true_counts[i] as f64 / totals.num_rows as f64
                };
                println!("{name} (BOOLEAN): entropia = {:.6}", binary_entropy(p));
            }
            DataType::Varchar => {
                println!(
                    "{name} (VARCHAR): znaków ASCII = {}",
                    totals.ascii_counts[i]
                );
            }
        }
    }
}
