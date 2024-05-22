use crate::{
    format::{AtomScheme, AtomValue},
    serde::ObjectScheme,
};

pub mod bin;
#[cfg(feature = "polars")]
pub mod polars;
pub mod text;

#[derive(Debug)]
struct OvShiftedHeader {
    header: Vec<AtomScheme>,
    column_shifting: Vec<usize>,
}
impl OvShiftedHeader {
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
