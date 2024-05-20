use std::marker::PhantomData;

use integer_encoding::{VarIntReader, VarIntWriter};

use crate::{
    format::{ObjectScheme, ObjectValue},
    serde::{OvDeserialize, OvSerialize},
};

#[derive(Debug)]
pub struct OvWriter<W, O> {
    has_written_header: bool,
    write: W,
    buf: Vec<u8>,
    _value: PhantomData<O>,
}
impl<W, V> OvWriter<W, V> {
    pub fn new(write: W) -> Self {
        Self {
            has_written_header: false,
            write,
            buf: vec![],
            _value: PhantomData,
        }
    }
}
impl<W, O> OvWriter<W, O>
where
    W: std::io::Write,
    O: OvSerialize,
{
    pub fn write(&mut self, object: &O) -> std::io::Result<()> {
        if self.has_written_header {
            self.has_written_header = true;

            let header = object.object_scheme();
            let header = bincode::serialize(&header).unwrap();
            self.write.write_varint(header.len())?;
            self.write.write_all(&header)?;
        }

        let mut atoms = vec![];
        object.serialize(&mut atoms);

        let object = ObjectValue::new(atoms);
        self.buf.clear();
        object.encode(&mut self.buf);

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
    scheme: Option<ObjectScheme>,
    read: R,
    buf: Vec<u8>,
    _value: PhantomData<O>,
}
impl<R, O> OvReader<R, O>
where
    R: std::io::Read,
    O: OvDeserialize,
{
    pub fn read(&mut self) -> std::io::Result<O> {
        let Some(scheme) = &self.scheme else {
            let len = self.read.read_varint()?;
            let mut buf = vec![0; len];
            self.read.read_exact(&mut buf)?;
            let scheme =
                bincode::deserialize(&buf).map_err(|_| std::io::ErrorKind::InvalidInput)?;
            self.scheme = Some(scheme);

            return self.read();
        };

        let len = self.read.read_varint()?;

        self.buf.clear();
        self.buf.extend(std::iter::repeat(0).take(len));
        let object = ObjectValue::decode(scheme, &mut std::io::Cursor::new(&self.buf))
            .ok_or(std::io::ErrorKind::InvalidInput)?;

        let object = O::deserialize(&mut object.atoms().as_slice()).unwrap();
        Ok(object)
    }
}
