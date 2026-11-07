//! End-to-end: zapis batchy do prawdziwego pliku na dysku i odczyt z powrotem.

mod common;

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use common::*;
use klakier::KlakierError;
use klakier::batch::{Batch, MAX_BATCH_SIZE};
use klakier::column::Column;
use klakier::file::{FileReader, FileWriter};
use klakier::schema::{DataType, Schema, TableColumn};

#[test]
fn single_batch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("single.klk");
    let schema = sample_schema();
    let batches = vec![sample_batch(&schema, 0, 100, 1)];

    write_file(&path, &schema, &batches).unwrap();
    let (read_schema, read_batches) = read_file(&path).unwrap();

    assert_eq!(*read_schema, *schema);
    assert_eq!(read_batches, batches);
}

#[test]
fn many_batches_with_partial_last() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many.klk");
    let schema = sample_schema();
    let batches = sample_batches(&schema, 5, 17);

    write_file(&path, &schema, &batches).unwrap();
    let (_, read_batches) = read_file(&path).unwrap();

    assert_eq!(read_batches, batches);
}

#[test]
fn empty_file_without_batches() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.klk");
    let schema = sample_schema();

    write_file(&path, &schema, &[]).unwrap();
    let (read_schema, read_batches) = read_file(&path).unwrap();

    assert_eq!(*read_schema, *schema);
    assert!(read_batches.is_empty());
}

#[test]
fn empty_batches_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("skip.klk");
    let schema = sample_schema();
    let empty = sample_batch(&schema, 0, 0, 0);
    let full = sample_batch(&schema, 0, 10, 0);

    write_file(&path, &schema, &[empty.clone(), full.clone(), empty]).unwrap();
    let (_, read_batches) = read_file(&path).unwrap();

    assert_eq!(read_batches, [full]);
}

#[test]
fn num_rows_from_footer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rows.klk");
    let schema = sample_schema();
    let batches = sample_batches(&schema, 3, 500);

    write_file(&path, &schema, &batches).unwrap();
    let reader = FileReader::open(BufReader::new(File::open(&path).unwrap())).unwrap();

    assert_eq!(reader.num_rows(), (2 * MAX_BATCH_SIZE + 500) as u64);
    assert_eq!(reader.num_row_groups(), 3);
}

#[test]
fn iterate_batches_one_by_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("iter.klk");
    let schema = sample_schema();
    let batches = sample_batches(&schema, 3, 100);

    write_file(&path, &schema, &batches).unwrap();
    let mut reader = FileReader::open(BufReader::new(File::open(&path).unwrap())).unwrap();
    let mut it = reader.batches();

    assert_eq!(it.next().unwrap().unwrap(), batches[0]);
    assert_eq!(it.next().unwrap().unwrap(), batches[1]);
    assert_eq!(it.next().unwrap().unwrap(), batches[2]);
    assert!(it.next().is_none());
}

#[test]
fn random_access_to_row_groups() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("random.klk");
    let schema = sample_schema();
    let batches = sample_batches(&schema, 4, 100);

    write_file(&path, &schema, &batches).unwrap();
    let mut reader = FileReader::open(BufReader::new(File::open(&path).unwrap())).unwrap();

    for i in (0..4).rev() {
        assert_eq!(reader.read_row_group(i).unwrap(), batches[i]);
    }
}

#[test]
fn single_column_types() {
    let dir = tempfile::tempdir().unwrap();
    let cases: Vec<(DataType, Column)> = vec![
        (
            DataType::Int64,
            Column::Int64(vec![i64::MIN, -1, 0, 1, i64::MAX]),
        ),
        (DataType::Bool, Column::Bool(vec![true; 9])),
        (
            DataType::Varchar,
            Column::Varchar(["zażółć", "gęślą", "", "jaźń 🐈"].into_iter().collect()),
        ),
    ];

    for (i, (data_type, column)) in cases.into_iter().enumerate() {
        let path = dir.path().join(format!("col{i}.klk"));
        let schema = Arc::new(Schema::new(vec![TableColumn::new("c", data_type)]));
        let batch = Batch::try_new(schema.clone(), vec![column]).unwrap();

        write_file(&path, &schema, std::slice::from_ref(&batch)).unwrap();
        let (_, read_batches) = read_file(&path).unwrap();

        assert_eq!(read_batches, [batch], "typ {data_type:?}");
    }
}

#[test]
fn write_batch_with_other_schema_fails() {
    let schema = sample_schema();
    let other = Arc::new(Schema::new(vec![TableColumn::new("x", DataType::Int64)]));
    let batch = Batch::try_new(other, vec![Column::Int64(vec![1])]).unwrap();

    let mut writer = FileWriter::new(Vec::new(), schema).unwrap();
    assert!(matches!(
        writer.write_batch(&batch),
        Err(KlakierError::SchemaMismatch)
    ));
}

#[test]
fn compresses_repetitive_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("compress.klk");
    let schema = Arc::new(Schema::new(vec![
        TableColumn::new("id", DataType::Int64),
        TableColumn::new("tekst", DataType::Varchar),
    ]));
    let text = "Klakier łapie smerfy. ".repeat(10);
    let batches: Vec<Batch> = (0..10)
        .map(|b| {
            let ids = (0..MAX_BATCH_SIZE as i64)
                .map(|i| b * 1_000_000 + i)
                .collect();
            let texts = (0..MAX_BATCH_SIZE).map(|_| text.as_str()).collect();
            Batch::try_new(
                schema.clone(),
                vec![Column::Int64(ids), Column::Varchar(texts)],
            )
            .unwrap()
        })
        .collect();

    write_file(&path, &schema, &batches).unwrap();

    let rows = 10 * MAX_BATCH_SIZE;
    let raw_size = rows * (8 + text.len());
    let file_size = std::fs::metadata(&path).unwrap().len() as usize;
    // rosnące id -> delta = 1 -> 1 bajt; powtarzalny tekst -> zstd
    assert!(
        file_size * 10 < raw_size,
        "plik {file_size} B, surowe dane {raw_size} B"
    );
    assert_eq!(read_file(&path).unwrap().1, batches);
}
