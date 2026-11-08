//! klakier — kolumnowy format pliku dla analitycznego DBMS.

pub mod batch;
pub mod column;
pub(crate) mod encoding;
pub mod error;
pub mod file;
pub mod schema;

pub use error::{KlakierError, Result};
