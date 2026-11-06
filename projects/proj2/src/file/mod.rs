//! Plik klakier (`.klk`): układ na dysku, zapis i odczyt.
//!
//! ```text
//! "KLAK"                          magic (4 B)
//! row group 0:
//!   chunk kolumny 0
//!   chunk kolumny 1
//!   ...
//! row group 1 ...
//! footer                          FileMeta::encode (schemat + położenie chunków)
//! footer_len                      u32 little-endian (4 B)
//! "KLAK"                          magic (4 B)
//! ```
//!
//! Writer zapisuje row groupy strumieniowo i dopiero na końcu footer.
//! Reader czyta najpierw ostatnie [`TRAILER_LEN`] bajtów, potem footer,
//! a następnie row groupy po kolei — w pamięci trzyma tylko jeden naraz.

pub mod meta;
pub mod reader;
pub mod writer;

pub use reader::FileReader;
pub use writer::FileWriter;

/// Magic bytes na początku i na końcu pliku.
pub const MAGIC: &[u8; 4] = b"KLAK";

/// Rozmiar końcówki pliku: footer_len (u32 LE) + MAGIC.
pub const TRAILER_LEN: usize = 4 + MAGIC.len();
