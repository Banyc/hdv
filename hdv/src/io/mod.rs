use crate::{
    format::{AtomScheme, AtomValue, ValueRow},
    serde::ObjectScheme,
};

pub mod bin;
#[cfg(feature = "polars")]
pub mod polars;
pub mod text;

#[derive(Debug)]
struct HdvShiftedHeader {
    header: Vec<AtomScheme>,
    column_shifting: Vec<usize>,
}
impl HdvShiftedHeader {
    pub fn new(header: Vec<AtomScheme>, object_scheme: &ObjectScheme) -> Option<Self> {
        let required = object_scheme.atom_schemes();
        let mut column_shifting = vec![];
        for required in &required {
            let i = header.iter().position(|x| x == required)?;
            column_shifting.push(i);
        }
        Some(Self {
            header,
            column_shifting,
        })
    }

    pub fn header(&self) -> &Vec<AtomScheme> {
        &self.header
    }

    pub fn shift(&self, source: &[Option<AtomValue>], values: &mut Vec<Option<AtomValue>>) {
        for i in self.column_shifting.iter().copied() {
            let value = source[i].clone();
            values.push(value);
        }
    }
}

fn assert_atom_types(header: &[AtomScheme], row: &ValueRow) {
    assert_eq!(header.len(), row.atoms().len());
    for (a, b) in header.iter().zip(row.atoms().iter()) {
        let Some(b) = b else {
            continue;
        };
        assert_eq!(a.r#type, b.into());
    }
}
