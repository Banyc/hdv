use std::marker::PhantomData;

use integer_encoding::{VarIntReader, VarIntWriter};

use crate::{
    format::{AtomScheme, ValueRow},
    serde::{OvDeserialize, OvScheme, OvSerialize},
};

#[derive(Debug)]
pub struct OvBinWriter<W, O> {
    has_written_header: bool,
    write: W,
    buf: Vec<u8>,
    _object: PhantomData<O>,
}
impl<W, V> OvBinWriter<W, V> {
    pub fn new(write: W) -> Self {
        Self {
            has_written_header: false,
            write,
            buf: vec![],
            _object: PhantomData,
        }
    }
}
impl<W, O> OvBinWriter<W, O>
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
pub struct OvBinReader<R, O> {
    header: Option<Vec<AtomScheme>>,
    read: R,
    buf: Vec<u8>,
    _object: PhantomData<O>,
}
impl<R, V> OvBinReader<R, V> {
    pub fn new(read: R) -> Self {
        Self {
            header: None,
            read,
            buf: vec![],
            _object: PhantomData,
        }
    }
}
impl<R, O> OvBinReader<R, O>
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
        format::{AtomType, AtomValue},
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
                            value: ValueType::Atom(AtomType::I64),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Atom(AtomType::F64),
                        },
                    ],
                }
            }
        }
        impl OvSerialize for A {
            fn serialize(&self, values: &mut Vec<Option<AtomValue>>) {
                values.push(Some(AtomValue::I64(self.a)));
                values.push(Some(AtomValue::F64(self.b)));
            }

            fn fill_nulls(values: &mut Vec<Option<AtomValue>>) {
                values.push(None);
                values.push(None);
            }
        }
        impl OvDeserialize for A {
            fn deserialize(values: &mut &[Option<AtomValue>]) -> Option<Self> {
                let a = {
                    let value = values.first()?.as_ref();
                    *values = &values[1..];
                    value
                };
                let b = {
                    let value = values.first()?.as_ref();
                    *values = &values[1..];
                    value
                };
                Some(Self {
                    a: a?.i64().unwrap() as _,
                    b: b?.f64().unwrap() as _,
                })
            }
        }

        let mut buf = vec![];
        let mut writer = OvBinWriter::new(&mut buf);
        let a = A { a: 1, b: 2. };
        let b = A { a: 3, b: 4. };
        writer.write(&a).unwrap();
        writer.write(&b).unwrap();
        writer.flush().unwrap();

        let mut reader = OvBinReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        let b_: A = reader.read().unwrap();
        assert_eq!(a, a_);
        assert_eq!(b, b_);
    }
}
