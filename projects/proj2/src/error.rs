//! Typ błędu dla zapisu i odczytu plików klakier.

use crate::batch::BatchError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum KlakierError {
    /// Błąd wejścia/wyjścia (otwarcie, odczyt, zapis pliku).
    Io(io::Error),
    /// Plik nie jest poprawnym plikiem klakier albo jest uszkodzony.
    Corrupted(&'static str),
    /// Z odczytanych danych nie da się zbudować poprawnego batcha.
    Batch(BatchError),
    /// Batch przekazany do writera ma inny schemat niż plik.
    SchemaMismatch,
}

pub type Result<T> = std::result::Result<T, KlakierError>;

impl fmt::Display for KlakierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KlakierError::Io(e) => write!(f, "błąd I/O: {e}"),
            KlakierError::Corrupted(what) => write!(f, "uszkodzony plik: {what}"),
            KlakierError::Batch(e) => write!(f, "niepoprawny batch: {e:?}"),
            KlakierError::SchemaMismatch => write!(f, "schemat batcha różni się od schematu pliku"),
        }
    }
}

impl std::error::Error for KlakierError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KlakierError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for KlakierError {
    fn from(e: io::Error) -> Self {
        KlakierError::Io(e)
    }
}

impl From<BatchError> for KlakierError {
    fn from(e: BatchError) -> Self {
        KlakierError::Batch(e)
    }
}
