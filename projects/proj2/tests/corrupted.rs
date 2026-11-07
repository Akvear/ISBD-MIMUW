mod common;

use std::fs::{self, OpenOptions};
use std::path::Path;

use common::*;
use klakier::KlakierError;

fn write_valid(path: &Path) -> Vec<u8> {
    let schema = sample_schema();
    let batches: Vec<_> = (0..3)
        .map(|i| sample_batch(&schema, i * 10, 5, i as u64))
        .collect();
    write_file(path, &schema, &batches).unwrap();
    fs::read(path).unwrap()
}

fn is_corrupted<T>(result: &klakier::Result<T>) -> bool {
    matches!(result, Err(KlakierError::Corrupted(_)))
}

#[test]
fn not_a_klakier_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tekst.klk");
    fs::write(&path, "To nie jest plik klakier, tylko zwykły tekst.").unwrap();

    assert!(is_corrupted(&read_file(&path)));
}

#[test]
fn every_truncation_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trunc.klk");
    let valid = write_valid(&path);

    for len in 0..valid.len() {
        let file = OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(len as u64).unwrap();
        drop(file);
        assert!(read_file(&path).is_err(), "ucięcie do {len} B przeszło");
        fs::write(&path, &valid).unwrap();
    }
}

#[test]
fn bad_magic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("magic.klk");
    let valid = write_valid(&path);

    let mut start = valid.clone();
    start[0] = b'X';
    fs::write(&path, &start).unwrap();
    assert!(is_corrupted(&read_file(&path)));

    let mut end = valid.clone();
    *end.last_mut().unwrap() = b'X';
    fs::write(&path, &end).unwrap();
    assert!(is_corrupted(&read_file(&path)));
}

#[test]
fn footer_len_larger_than_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("footer_len.klk");
    let mut bytes = write_valid(&path);

    let n = bytes.len();
    bytes[n - 8..n - 4].copy_from_slice(&u32::MAX.to_le_bytes());
    fs::write(&path, &bytes).unwrap();

    assert!(is_corrupted(&read_file(&path)));
}
