use std::marker::PhantomData;

use integer_encoding::{VarIntReader, VarIntWriter};

use crate::{
    format::{AtomScheme, AtomValue, ValueRow},
    serde::{HdvDeserialize, HdvScheme, HdvSerialize},
};

use super::{assert_atom_types, HdvShiftedHeader};

#[derive(Debug)]
pub struct HdvBinWriter<W, O> {
    has_written_header: bool,
    write: W,
    buf: Vec<u8>,
    _object: PhantomData<O>,
}
impl<W, O> HdvBinWriter<W, O> {
    pub fn new(write: W) -> Self {
        Self {
            has_written_header: false,
            write,
            buf: vec![],
            _object: PhantomData,
        }
    }
}
impl<W, O> HdvBinWriter<W, O>
where
    W: std::io::Write,
    O: HdvSerialize + HdvScheme,
{
    pub fn write(&mut self, object: &O) -> std::io::Result<()> {
        if !self.has_written_header {
            self.has_written_header = true;

            let header = O::object_scheme();
            let header = header.atom_schemes();
            write_header(&mut self.write, &header)?;
        }

        let mut atoms = vec![];
        object.serialize(&mut atoms);

        let row = ValueRow::new(atoms);
        write_row(&mut self.write, row, &mut self.buf)?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}

#[derive(Debug)]
pub struct HdvBinRawWriter<W> {
    header: Vec<AtomScheme>,
    has_written_header: bool,
    write: W,
    buf: Vec<u8>,
}
impl<W> HdvBinRawWriter<W> {
    pub fn new(write: W, header: Vec<AtomScheme>) -> Self {
        Self {
            header,
            has_written_header: false,
            write,
            buf: vec![],
        }
    }
}
impl<W> HdvBinRawWriter<W>
where
    W: std::io::Write,
{
    pub fn write(&mut self, row: ValueRow) -> std::io::Result<()> {
        if !self.has_written_header {
            self.has_written_header = true;

            write_header(&mut self.write, &self.header)?;
        }

        assert_atom_types(&self.header, &row);

        write_row(&mut self.write, row, &mut self.buf)?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}

#[derive(Debug)]
pub struct HdvBinReader<R, O> {
    shift_header: Option<HdvShiftedHeader>,
    read: R,
    buf: Vec<u8>,
    atom_value_buf: Vec<Option<AtomValue>>,
    _object: PhantomData<O>,
}
impl<R, V> HdvBinReader<R, V> {
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
impl<R, O> HdvBinReader<R, O>
where
    R: std::io::Read,
    O: HdvDeserialize + HdvScheme,
{
    pub fn read(&mut self) -> std::io::Result<O> {
        let Some(shift_header) = &self.shift_header else {
            let header = read_header(&mut self.read)?;
            let shift_header = HdvShiftedHeader::new(header, &O::object_scheme())
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
pub struct HdvBinRawReader<R> {
    header: Option<Vec<AtomScheme>>,
    read: R,
    buf: Vec<u8>,
}
impl<R> HdvBinRawReader<R> {
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
impl<R> HdvBinRawReader<R>
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

fn write_header<W>(write: &mut W, header: &[AtomScheme]) -> std::io::Result<()>
where
    W: std::io::Write,
{
    let header = bincode::serialize(header).unwrap();
    write.write_varint(header.len())?;
    write.write_all(&header)?;
    Ok(())
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

fn write_row<W>(write: &mut W, row: ValueRow, buf: &mut Vec<u8>) -> std::io::Result<()>
where
    W: std::io::Write,
{
    buf.clear();
    row.encode(buf);

    write.write_varint(buf.len())?;
    write.write_all(buf)?;
    Ok(())
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
        impl HdvScheme for A {
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
        impl HdvSerialize for A {
            fn serialize(&self, values: &mut Vec<Option<AtomValue>>) {
                values.push(Some(AtomValue::I64(self.a)));
                values.push(Some(AtomValue::F64(self.b)));
            }

            fn fill_nulls(values: &mut Vec<Option<AtomValue>>) {
                values.push(None);
                values.push(None);
            }
        }
        impl HdvDeserialize for A {
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
        let mut writer = HdvBinWriter::new(&mut buf);
        let a = A { a: 1, b: 2. };
        let b = A { a: 3, b: 4. };
        writer.write(&a).unwrap();
        writer.write(&b).unwrap();
        writer.flush().unwrap();

        let mut reader = HdvBinReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        let b_: A = reader.read().unwrap();
        assert_eq!(a, a_);
        assert_eq!(b, b_);

        let mut reader = HdvBinRawReader::new(std::io::Cursor::new(&buf));
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

        let mut buf_ = vec![];
        let header = A::object_scheme().atom_schemes().clone();
        let mut writer = HdvBinRawWriter::new(&mut buf_, header);
        writer
            .write(ValueRow::new(vec![
                Some(AtomValue::I64(1)),
                Some(AtomValue::F64(2.0)),
            ]))
            .unwrap();
        writer
            .write(ValueRow::new(vec![
                Some(AtomValue::I64(3)),
                Some(AtomValue::F64(4.0)),
            ]))
            .unwrap();
        writer.flush().unwrap();
        assert_eq!(buf, buf_);
    }
}
