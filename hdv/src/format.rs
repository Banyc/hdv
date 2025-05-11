use std::{
    io::{Read, Write},
    sync::Arc,
};

use integer_encoding::{FixedIntReader, FixedIntWriter, VarIntReader, VarIntWriter};
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct AtomScheme {
    pub name: String,
    pub r#type: AtomType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueRow {
    atoms: Vec<Option<AtomValue>>,
}
impl ValueRow {
    pub fn new(atoms: Vec<Option<AtomValue>>) -> Self {
        Self { atoms }
    }

    pub fn atoms(&self) -> &Vec<Option<AtomValue>> {
        &self.atoms
    }

    pub fn into_atoms(self) -> Vec<Option<AtomValue>> {
        self.atoms
    }

    pub fn encode(&self, buf: &mut Vec<u8>) {
        let mut num_cont_somes: usize = 0;
        for (i, atom) in self.atoms.iter().enumerate() {
            if num_cont_somes == 0 {
                for atom in &self.atoms[i..] {
                    if atom.is_none() {
                        break;
                    }
                    num_cont_somes += 1;
                }
                buf.write_varint(num_cont_somes).unwrap();
            }
            if num_cont_somes == 0 {
                assert!(atom.is_none());
                continue;
            }
            atom.as_ref().unwrap().encode(buf);
            num_cont_somes -= 1;
        }
    }
    pub fn decode(atom_schemes: &[AtomScheme], buf: &mut std::io::Cursor<&[u8]>) -> Option<Self> {
        let mut atoms = vec![];
        let mut num_cont_somes: usize = 0;
        for ty in atom_schemes.iter().map(|x| x.r#type) {
            if num_cont_somes == 0 {
                num_cont_somes = buf.read_varint().ok()?;
            }
            if num_cont_somes == 0 {
                atoms.push(None);
                continue;
            }
            let atom = AtomValue::decode(ty, buf)?;
            atoms.push(Some(atom));
            num_cont_somes -= 1;
        }
        Some(Self { atoms })
    }
}

#[derive(Debug, Clone, PartialEq, EnumDiscriminants, bincode::Encode, bincode::Decode)]
#[strum_discriminants(derive(Serialize, Deserialize, bincode::Encode, bincode::Decode))]
#[strum_discriminants(name(AtomType))]
pub enum AtomValue {
    String(Arc<str>),
    Bytes(Arc<[u8]>),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
}
impl AtomValue {
    pub fn string(&self) -> Option<&Arc<str>> {
        let Self::String(x) = self else {
            return None;
        };
        Some(x)
    }
    pub fn bytes(&self) -> Option<&Arc<[u8]>> {
        let Self::Bytes(x) = self else {
            return None;
        };
        Some(x)
    }
    pub fn u64(&self) -> Option<u64> {
        let Self::U64(x) = self else {
            return None;
        };
        Some(*x)
    }
    pub fn i64(&self) -> Option<i64> {
        let Self::I64(x) = self else {
            return None;
        };
        Some(*x)
    }
    pub fn f32(&self) -> Option<f32> {
        let Self::F32(x) = self else {
            return None;
        };
        Some(*x)
    }
    pub fn f64(&self) -> Option<f64> {
        let Self::F64(x) = self else {
            return None;
        };
        Some(*x)
    }
    pub fn bool(&self) -> Option<bool> {
        let Self::Bool(x) = self else {
            return None;
        };
        Some(*x)
    }

    const BOOL_FALSE: u8 = 0;
    const BOOL_TRUE: u8 = 1;

    pub fn encode(&self, buf: &mut Vec<u8>) {
        match &self {
            AtomValue::String(x) => {
                let bytes = x.as_bytes();
                buf.write_varint(bytes.len()).unwrap();
                buf.write_all(bytes).unwrap();
            }
            AtomValue::Bytes(x) => {
                buf.write_varint(x.len()).unwrap();
                buf.write_all(x).unwrap();
            }
            AtomValue::U64(x) => {
                buf.write_varint(*x).unwrap();
            }
            AtomValue::I64(x) => {
                buf.write_varint(*x).unwrap();
            }
            AtomValue::F32(x) => {
                buf.write_fixedint(x.to_bits()).unwrap();
            }
            AtomValue::F64(x) => {
                buf.write_fixedint(x.to_bits()).unwrap();
            }
            AtomValue::Bool(x) => {
                buf.write_fixedint(if *x {
                    Self::BOOL_TRUE
                } else {
                    Self::BOOL_FALSE
                })
                .unwrap();
            }
        }
    }

    pub fn decode(ty: AtomType, buf: &mut std::io::Cursor<&[u8]>) -> Option<Self> {
        match ty {
            AtomType::String => {
                let len: usize = buf.read_varint().ok()?;
                let mut bytes = vec![0; len];
                buf.read_exact(&mut bytes).ok()?;
                Some(Self::String(String::from_utf8(bytes).ok()?.into()))
            }
            AtomType::Bytes => {
                let len: usize = buf.read_varint().ok()?;
                let mut bytes = vec![0; len];
                buf.read_exact(&mut bytes).ok()?;
                Some(Self::Bytes(bytes.into()))
            }
            AtomType::U64 => {
                let x: u64 = buf.read_varint().ok()?;
                Some(Self::U64(x))
            }
            AtomType::I64 => {
                let x: i64 = buf.read_varint().ok()?;
                Some(Self::I64(x))
            }
            AtomType::F32 => {
                let bits: u32 = buf.read_fixedint().ok()?;
                Some(Self::F32(f32::from_bits(bits)))
            }
            AtomType::F64 => {
                let bits: u64 = buf.read_fixedint().ok()?;
                Some(Self::F64(f64::from_bits(bits)))
            }
            AtomType::Bool => {
                let bit: u8 = buf.read_fixedint().ok()?;
                Some(Self::Bool(match bit {
                    Self::BOOL_FALSE => false,
                    Self::BOOL_TRUE => true,
                    _ => return None,
                }))
            }
        }
    }
}
