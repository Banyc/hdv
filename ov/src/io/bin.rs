use std::marker::PhantomData;

use integer_encoding::{VarIntReader, VarIntWriter};

use crate::{
    format::{AtomScheme, AtomValue, ValueRow},
    serde::{OvDeserialize, OvScheme, OvSerialize},
};

use super::OvShiftedHeader;

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
    shift_header: Option<OvShiftedHeader>,
    read: R,
    buf: Vec<u8>,
    atom_value_buf: Vec<Option<AtomValue>>,
    _object: PhantomData<O>,
}
impl<R, V> OvBinReader<R, V> {
    pub fn new(read: R) -> Self {
        Self {
            shift_header: None,
            read,
            buf: vec![],
            atom_value_buf: vec![],
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
        let Some(shift_header) = &self.shift_header else {
            let header = read_header(&mut self.read)?;
            let shift_header = OvShiftedHeader::new(header, &O::object_scheme())
                .ok_or(std::io::ErrorKind::InvalidInput)?;
            self.shift_header = Some(shift_header);

            return self.read();
        };

        let row = read_row(&mut self.read, shift_header.header(), &mut self.buf)?;
        self.atom_value_buf.clear();
        shift_header.shift(row.atoms(), &mut self.atom_value_buf);

        let object = O::deserialize(&mut self.atom_value_buf.as_slice()).unwrap();
        Ok(object)
    }
}

#[derive(Debug)]
pub struct OvBinRawReader<R> {
    header: Option<Vec<AtomScheme>>,
    read: R,
    buf: Vec<u8>,
}
impl<R> OvBinRawReader<R> {
    pub fn new(read: R) -> Self {
        Self {
            header: None,
            read,
            buf: vec![],
        }
    }

    pub fn header(&self) -> Option<&Vec<AtomScheme>> {
        self.header.as_ref()
    }
}
impl<R> OvBinRawReader<R>
where
    R: std::io::Read,
{
    pub fn read(&mut self) -> std::io::Result<ValueRow> {
        let Some(header) = &self.header else {
            self.header = Some(read_header(&mut self.read)?);

            return self.read();
        };

        let row = read_row(&mut self.read, header, &mut self.buf)?;
        Ok(row)
    }
}

fn read_header<R>(read: &mut R) -> std::io::Result<Vec<AtomScheme>>
where
    R: std::io::Read,
{
    let len = read.read_varint()?;
    let mut buf = vec![0; len];
    read.read_exact(&mut buf)?;
    let header: Vec<AtomScheme> =
        bincode::deserialize(&buf).map_err(|_| std::io::ErrorKind::InvalidInput)?;
    Ok(header)
}
fn read_row<R>(
    read: &mut R,
    atom_schemes: &[AtomScheme],
    buf: &mut Vec<u8>,
) -> std::io::Result<ValueRow>
where
    R: std::io::Read,
{
    let len = read.read_varint()?;

    buf.clear();
    buf.extend(std::iter::repeat(0).take(len));
    read.read_exact(buf)?;
    let row = ValueRow::decode(atom_schemes, &mut std::io::Cursor::new(buf))
        .ok_or(std::io::ErrorKind::InvalidInput)?;
    Ok(row)
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

        let mut reader = OvBinRawReader::new(std::io::Cursor::new(&buf));
        let a_ = reader.read().unwrap();
        let b_ = reader.read().unwrap();
        assert_eq!(
            a_.atoms().as_slice(),
            [Some(AtomValue::I64(1)), Some(AtomValue::F64(2.0))]
        );
        assert_eq!(
            b_.atoms().as_slice(),
            [Some(AtomValue::I64(3)), Some(AtomValue::F64(4.0))]
        );
    }
}
