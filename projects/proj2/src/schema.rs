//! Schemat danych: nazwy i typy kolumn.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Int64,
    Bool,
    Varchar,
}

/// Opis pojedynczej kolumny.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    pub name: String,
    pub data_type: DataType,
}

impl TableColumn {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub table_columns: Vec<TableColumn>,
}

impl Schema {
    pub fn new(table_columns: Vec<TableColumn>) -> Self {
        Self { table_columns }
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.table_columns.iter().position(|f| f.name == name)
    }
}
