use std::marker::PhantomData;

use crate::{
    format::{AtomType, AtomValue, ValueRow},
    serde::{OvDeserialize, OvScheme, OvSerialize},
};

use super::OvShiftedHeader;

#[derive(Debug)]
pub struct OvTextWriterOptions {
    pub is_csv_header: bool,
}
#[derive(Debug)]
pub struct OvTextWriter<W, O> {
    options: OvTextWriterOptions,
    has_written_header: bool,
    write: W,
    _object: PhantomData<O>,
}
impl<W, V> OvTextWriter<W, V> {
    pub fn new(write: W, options: OvTextWriterOptions) -> Self {
        Self {
            options,
            has_written_header: false,
            write,
            _object: PhantomData,
        }
    }
}
impl<W, O> OvTextWriter<W, O>
where
    W: std::io::Write,
    O: OvSerialize + OvScheme,
{
    pub fn write(&mut self, object: &O) -> std::io::Result<()> {
        if !self.has_written_header {
            self.has_written_header = true;

            let header = O::object_scheme();
            let header = header.atom_schemes();
            if self.options.is_csv_header {
                for item in &header {
                    self.write.write_all(item.name.as_bytes())?;
                    self.write.write_all(b",")?;
                }
            } else {
                let header = ron::to_string(&header).unwrap();
                self.write.write_all(header.as_bytes())?;
            }
            self.write.write_all(b"\n").unwrap();
        }

        let mut atoms = vec![];
        object.serialize(&mut atoms);

        let row = ValueRow::new(atoms);
        for item in row.atoms() {
            let Some(value) = item else {
                self.write.write_all(b",")?;
                continue;
            };
            match value {
                AtomValue::String(x) => {
                    if x.contains(",")
                        || x.contains("\"")
                        || x.contains("\n")
                        || x.trim_ascii_start() != x
                    {
                        return Err(std::io::ErrorKind::InvalidInput)?;
                    }
                    self.write.write_all(x.as_bytes())?;
                    self.write.write_all(b",")?;
                }
                AtomValue::Bytes(_) => {
                    return Err(std::io::ErrorKind::InvalidInput)?;
                }
                AtomValue::U64(x) => {
                    self.write.write_all(format!("{x},").as_bytes())?;
                }
                AtomValue::I64(x) => {
                    self.write.write_all(format!("{x},").as_bytes())?;
                }
                AtomValue::F32(x) => {
                    self.write.write_all(format!("{x},").as_bytes())?;
                }
                AtomValue::F64(x) => {
                    self.write.write_all(format!("{x},").as_bytes())?;
                }
            }
        }
        self.write.write_all(b"\n")?;
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}

#[derive(Debug)]
pub struct OvTextReader<R, O> {
    shift_header: Option<OvShiftedHeader>,
    read: R,
    buf: String,
    atom_value_buf: Vec<Option<AtomValue>>,
    _object: PhantomData<O>,
}
impl<R, V> OvTextReader<R, V> {
    pub fn new(read: R) -> Self {
        Self {
            shift_header: None,
            read,
            buf: String::new(),
            atom_value_buf: vec![],
            _object: PhantomData,
        }
    }
}
impl<R, O> OvTextReader<R, O>
where
    R: std::io::BufRead,
    O: OvDeserialize + OvScheme,
{
    pub fn read(&mut self) -> std::io::Result<O> {
        let Some(shift_header) = &self.shift_header else {
            self.buf.clear();
            self.read.read_line(&mut self.buf)?;
            let header = ron::from_str(&self.buf).map_err(|_| std::io::ErrorKind::InvalidInput)?;
            let shift_header = OvShiftedHeader::new(header, &O::object_scheme())
                .ok_or(std::io::ErrorKind::InvalidInput)?;
            self.shift_header = Some(shift_header);

            return self.read();
        };

        self.buf.clear();
        self.read.read_line(&mut self.buf)?;
        let items = self.buf.split(",");
        let zip = items.zip(shift_header.header().iter());
        let mut atoms = vec![];
        for (item, scheme) in zip {
            if item.split_whitespace().next().is_none() {
                atoms.push(None);
                continue;
            }
            let atom = match scheme.value {
                AtomType::String => AtomValue::String(item.trim_start().to_string()),
                AtomType::Bytes => return Err(std::io::ErrorKind::InvalidInput)?,
                AtomType::U64 => AtomValue::U64(
                    item.trim()
                        .parse()
                        .map_err(|_| std::io::ErrorKind::InvalidInput)?,
                ),
                AtomType::I64 => AtomValue::I64(
                    item.trim()
                        .parse()
                        .map_err(|_| std::io::ErrorKind::InvalidInput)?,
                ),
                AtomType::F32 => AtomValue::F32(
                    item.trim()
                        .parse()
                        .map_err(|_| std::io::ErrorKind::InvalidInput)?,
                ),
                AtomType::F64 => AtomValue::F64(
                    item.trim()
                        .parse()
                        .map_err(|_| std::io::ErrorKind::InvalidInput)?,
                ),
            };
            atoms.push(Some(atom));
        }
        self.atom_value_buf.clear();
        shift_header.shift(&atoms, &mut self.atom_value_buf);

        let object = O::deserialize(&mut self.atom_value_buf.as_slice()).unwrap();
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
        let options = OvTextWriterOptions {
            is_csv_header: false,
        };
        let mut writer = OvTextWriter::new(&mut buf, options);
        let a = A { a: 1, b: 2. };
        let b = A { a: 3, b: 4. };
        writer.write(&a).unwrap();
        writer.write(&b).unwrap();
        writer.flush().unwrap();
        println!("{}", String::from_utf8(buf.clone()).unwrap());

        let mut reader = OvTextReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        let b_: A = reader.read().unwrap();
        assert_eq!(a, a_);
        assert_eq!(b, b_);

        let mut buf = vec![];
        let options = OvTextWriterOptions {
            is_csv_header: true,
        };
        let mut writer = OvTextWriter::new(&mut buf, options);
        let a = A { a: 1, b: 2. };
        let b = A { a: 3, b: 4. };
        writer.write(&a).unwrap();
        writer.write(&b).unwrap();
        writer.flush().unwrap();
        println!("{}", String::from_utf8(buf.clone()).unwrap());
    }
}
