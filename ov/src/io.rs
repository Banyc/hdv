use std::marker::PhantomData;

use integer_encoding::{VarIntReader, VarIntWriter};

use crate::{
    format::{AtomScheme, ValueRow},
    serde::{OvDeserialize, OvScheme, OvSerialize},
};

#[derive(Debug)]
pub struct OvWriter<W, O> {
    has_written_header: bool,
    write: W,
    buf: Vec<u8>,
    _object: PhantomData<O>,
}
impl<W, V> OvWriter<W, V> {
    pub fn new(write: W) -> Self {
        Self {
            has_written_header: false,
            write,
            buf: vec![],
            _object: PhantomData,
        }
    }
}
impl<W, O> OvWriter<W, O>
where
    W: std::io::Write,
    O: OvSerialize + OvScheme,
{
    pub fn write(&mut self, object: &O) -> std::io::Result<()> {
        if !self.has_written_header {
            self.has_written_header = true;

            let header = O::object_scheme();
            let header = header.atom_schemes();
            let header = bincode::serialize(&header).unwrap();
            self.write.write_varint(header.len())?;
            self.write.write_all(&header)?;
        }

        let mut atoms = vec![];
        object.serialize(&mut atoms);

        let row = ValueRow::new(atoms);
        self.buf.clear();
        row.encode(&mut self.buf);

        self.write.write_varint(self.buf.len())?;
        self.write.write_all(&self.buf)?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}

#[derive(Debug)]
pub struct OvReader<R, O> {
    header: Option<Vec<AtomScheme>>,
    read: R,
    buf: Vec<u8>,
    _object: PhantomData<O>,
}
impl<R, V> OvReader<R, V> {
    pub fn new(read: R) -> Self {
        Self {
            header: None,
            read,
            buf: vec![],
            _object: PhantomData,
        }
    }
}
impl<R, O> OvReader<R, O>
where
    R: std::io::Read,
    O: OvDeserialize + OvScheme,
{
    pub fn read(&mut self) -> std::io::Result<O> {
        let Some(header) = &self.header else {
            let len = self.read.read_varint()?;
            let mut buf = vec![0; len];
            self.read.read_exact(&mut buf)?;
            let header =
                bincode::deserialize(&buf).map_err(|_| std::io::ErrorKind::InvalidInput)?;
            if header != O::object_scheme().atom_schemes() {
                return Err(std::io::ErrorKind::InvalidInput)?;
            }
            self.header = Some(header);

            return self.read();
        };

        let len = self.read.read_varint()?;

        self.buf.clear();
        self.buf.extend(std::iter::repeat(0).take(len));
        self.read.read_exact(&mut self.buf)?;
        let row = ValueRow::decode(header, &mut std::io::Cursor::new(&self.buf))
            .ok_or(std::io::ErrorKind::InvalidInput)?;

        let object = O::deserialize(&mut row.atoms().as_slice()).unwrap();
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        format::{AtomOptionType, AtomOptionValue, AtomType, AtomValue},
        serde::{FieldScheme, ObjectScheme, ValueType},
    };

    use super::*;

    #[test]
    fn test_io() {
        #[derive(Debug, PartialEq)]
        struct A {
            a: i64,
            b: f64,
        }
        impl OvScheme for A {
            fn object_scheme() -> ObjectScheme {
                ObjectScheme {
                    fields: vec![
                        FieldScheme {
                            name: "a".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::I64,
                                nullable: false,
                            }),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::F64,
                                nullable: false,
                            }),
                        },
                    ],
                }
            }
        }
        impl OvSerialize for A {
            fn serialize(&self, values: &mut Vec<AtomOptionValue>) {
                values.push(AtomOptionValue::Solid(AtomValue::I64(self.a)));
                values.push(AtomOptionValue::Solid(AtomValue::F64(self.b)));
            }
        }
        impl OvDeserialize for A {
            fn deserialize(values: &mut &[AtomOptionValue]) -> Option<Self> {
                Some(Self {
                    a: {
                        let value = values.first()?.atom_value()?.i64()? as _;
                        *values = &values[1..];
                        value
                    },
                    b: {
                        let value = values.first()?.atom_value()?.f64()?;
                        *values = &values[1..];
                        value
                    },
                })
            }
        }

        let mut buf = vec![];
        let mut writer = OvWriter::new(&mut buf);
        let a = A { a: 1, b: 2. };
        let b = A { a: 3, b: 4. };
        writer.write(&a).unwrap();
        writer.write(&b).unwrap();
        writer.flush().unwrap();

        let mut reader = OvReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        let b_: A = reader.read().unwrap();
        assert_eq!(a, a_);
        assert_eq!(b, b_);
    }
}
