use std::io::{Read, Write};

use integer_encoding::{FixedIntReader, FixedIntWriter, VarIntReader, VarIntWriter};
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectScheme {
    pub fields: Vec<FieldScheme>,
}
impl ObjectScheme {
    pub fn atom_schemes(&self) -> Vec<AtomScheme> {
        let mut atoms = vec![];
        for field in &self.fields {
            atoms.extend(field.atom_schemes().into_iter());
        }
        atoms
    }

    pub fn atom_types(&self, types: &mut Vec<AtomOptionType>) {
        for field in &self.fields {
            field.atom_types(types);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldScheme {
    pub name: String,
    pub value: ValueType,
}
impl FieldScheme {
    pub fn atom_schemes(&self) -> Vec<AtomScheme> {
        let post_atoms = match &self.value {
            ValueType::Atom(x) => {
                return vec![AtomScheme {
                    name: self.name.clone(),
                    value: *x,
                }]
            }
            ValueType::Object(object) => object.atom_schemes(),
        };
        let mut atoms = vec![];
        for post_atom in &post_atoms {
            let name = format!("{}.{}", self.name, post_atom.name);
            atoms.push(AtomScheme {
                name,
                value: post_atom.value,
            });
        }
        atoms
    }

    pub fn atom_types(&self, types: &mut Vec<AtomOptionType>) {
        match &self.value {
            ValueType::Atom(x) => types.push(*x),
            ValueType::Object(object) => object.atom_types(types),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    Atom(AtomOptionType),
    Object(ObjectScheme),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomScheme {
    pub name: String,
    pub value: AtomOptionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomOptionType {
    pub value: AtomType,
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectValue {
    atoms: Vec<AtomOptionValue>,
}
impl ObjectValue {
    pub fn new(atoms: Vec<AtomOptionValue>) -> Self {
        Self { atoms }
    }

    pub fn atoms(&self) -> &Vec<AtomOptionValue> {
        &self.atoms
    }

    const IS_NONE: u8 = 0;
    const IS_SOME: u8 = 1;

    pub fn encode(&self, buf: &mut Vec<u8>) {
        for atom in &self.atoms {
            let atom = match atom {
                AtomOptionValue::Solid(x) => x,
                AtomOptionValue::Option(x) => match x {
                    Some(x) => {
                        buf.write_fixedint(Self::IS_SOME).unwrap();
                        x
                    }
                    None => {
                        buf.write_fixedint(Self::IS_NONE).unwrap();
                        continue;
                    }
                },
            };
            atom.encode(buf);
        }
    }

    pub fn decode(scheme: &ObjectScheme, buf: &mut std::io::Cursor<&[u8]>) -> Option<Self> {
        let mut atom_types = vec![];
        scheme.atom_types(&mut atom_types);
        let mut atoms = vec![];
        for ty in atom_types {
            if ty.nullable {
                let is_some: u8 = buf.read_fixedint().ok()?;
                match is_some {
                    Self::IS_NONE => {
                        atoms.push(AtomOptionValue::Option(None));
                        continue;
                    }
                    Self::IS_SOME => (),
                    _ => return None,
                }
            }
            let atom = AtomValue::decode(ty.value, buf)?;
            let atom = if ty.nullable {
                AtomOptionValue::Option(Some(atom))
            } else {
                AtomOptionValue::Solid(atom)
            };
            atoms.push(atom);
        }
        Some(Self { atoms })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AtomOptionValue {
    Solid(AtomValue),
    Option(Option<AtomValue>),
}
impl AtomOptionValue {
    pub fn atom_value(&self) -> Option<&AtomValue> {
        match self {
            AtomOptionValue::Solid(x) => Some(x),
            AtomOptionValue::Option(x) => x.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[strum_discriminants(name(AtomType))]
pub enum AtomValue {
    Bytes(Vec<u8>),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
}
impl AtomValue {
    pub fn bytes(&self) -> Option<&Vec<u8>> {
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

    pub fn encode(&self, buf: &mut Vec<u8>) {
        match &self {
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
        }
    }

    pub fn decode(ty: AtomType, buf: &mut std::io::Cursor<&[u8]>) -> Option<Self> {
        match ty {
            AtomType::Bytes => {
                let len: usize = buf.read_varint().ok()?;
                let mut bytes = vec![0; len];
                buf.read_exact(&mut bytes).ok()?;
                Some(Self::Bytes(bytes))
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
        }
    }
}
