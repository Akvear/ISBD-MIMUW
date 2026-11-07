//! Reprezentacja kolumn w pamięci.

use crate::schema::DataType;

/// Pojedyncza kolumna danych.
#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    Int64(Vec<i64>),
    Bool(Vec<bool>),
    Varchar(Varchar),
}

impl Column {
    /// Liczba wierszy w kolumnie.
    pub fn len(&self) -> usize {
        match self {
            Column::Int64(values) => values.len(),
            Column::Bool(values) => values.len(),
            Column::Varchar(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn data_type(&self) -> DataType {
        match self {
            Column::Int64(_) => DataType::Int64,
            Column::Bool(_) => DataType::Bool,
            Column::Varchar(_) => DataType::Varchar,
        }
    }

    pub fn as_int64(&self) -> Option<&[i64]> {
        match self {
            Column::Int64(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<&[bool]> {
        match self {
            Column::Bool(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_varchar(&self) -> Option<&Varchar> {
        match self {
            Column::Varchar(values) => Some(values),
            _ => None,
        }
    }
}

/// Kolumna napisów offsets + data.
#[derive(Debug, Clone, PartialEq)]
pub struct Varchar {
    pub offsets: Vec<u32>,
    pub data: Vec<u8>,
}

impl Varchar {
    pub fn new() -> Self {
        Self {
            offsets: vec![0],
            data: Vec::new(),
        }
    }

    /// Liczba napisów.
    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Dopisuje napis na koniec kolumny.
    ///
    /// Panikuje, jeśli łączny rozmiar `data` przekroczyłby `u32::MAX`.
    pub fn push(&mut self, value: &[u8]) {
        self.data.extend_from_slice(value);
        let end = u32::try_from(self.data.len()).expect("Varchar: data przekracza u32::MAX bajtów");
        self.offsets.push(end);
    }

    /// Napis o indeksie `i`.
    ///
    /// Panikuje, jeśli `i >= len()`.
    pub fn get(&self, i: usize) -> &[u8] {
        let start = self.offsets[i] as usize;
        let end = self.offsets[i + 1] as usize;
        &self.data[start..end]
    }

    /// Iterator po napisach, w kolejności wierszy.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.offsets
            .iter()
            .zip(self.offsets.iter().skip(1))
            .map(|(&start, &end)| &self.data[start as usize..end as usize])
    }
}

impl<S: AsRef<[u8]>> FromIterator<S> for Varchar {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        let mut column = Varchar::new();
        for s in iter {
            column.push(s.as_ref());
        }
        column
    }
}

impl Default for Varchar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varchar_push_get() {
        let mut v = Varchar::new();
        assert!(v.is_empty());
        v.push(b"ab");
        v.push(b"");
        v.push("żółw".as_bytes());
        assert_eq!(v.len(), 3);
        assert_eq!(v.get(0), b"ab");
        assert_eq!(v.get(1), b"");
        assert_eq!(v.get(2), "żółw".as_bytes());
        assert_eq!(v.offsets, [0, 2, 2, 9]);
    }

    #[test]
    fn varchar_from_iter_and_iter() {
        let v = Varchar::from_iter(["ab", "", "żółw"]);
        assert_eq!(v.offsets, [0, 2, 2, 9]);
        let owned: Varchar = vec![String::from("ab"), String::new(), "żółw".into()]
            .into_iter()
            .collect();
        assert_eq!(v, owned);

        let back: Vec<&str> = v.iter().map(|s| std::str::from_utf8(s).unwrap()).collect();
        assert_eq!(back, ["ab", "", "żółw"]);
        assert_eq!(Varchar::from_iter(Vec::<&str>::new()).iter().len(), 0);
    }

    #[test]
    #[should_panic]
    fn varchar_get_out_of_bounds() {
        Varchar::new().get(0);
    }
}
