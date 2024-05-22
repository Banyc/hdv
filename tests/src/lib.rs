#[cfg(test)]
mod tests {
    use ov::{
        format::AtomValue,
        io::bin::{OvBinReader, OvBinWriter},
        serde::{OvDeserialize, OvSerialize},
    };
    use ov_derive::OvSerde;

    #[test]
    fn test_derive() {
        #[derive(Debug, OvSerde, PartialEq)]
        pub struct PartialA {
            c: Option<f64>,
            a: u16,
        }

        #[derive(Debug, OvSerde, PartialEq)]
        pub struct A {
            a: u16,
            b: Option<B>,
            c: Option<f64>,
            d: B,
        }
        #[derive(Debug, OvSerde, PartialEq)]
        struct B {
            a: Vec<u8>,
            b: i64,
            c: String,
            d: Option<Vec<u8>>,
        }

        let a = A {
            a: 1,
            b: None,
            c: Some(3.),
            d: B {
                a: b"hello".to_vec(),
                b: 2,
                c: "world".to_owned(),
                d: None,
            },
        };

        let mut values = vec![];
        a.serialize(&mut values);
        assert_eq!(
            values,
            [
                Some(AtomValue::U64(1)),
                None,
                None,
                None,
                None,
                Some(AtomValue::F64(3.0)),
                Some(AtomValue::Bytes(b"hello".to_vec())),
                Some(AtomValue::I64(2)),
                Some(AtomValue::String("world".to_owned())),
                None,
            ]
        );

        let b = A::deserialize(&mut &*values).unwrap();
        assert_eq!(a, b);

        let mut buf = vec![];
        let mut writer = OvBinWriter::new(&mut buf);
        writer.write(&a).unwrap();
        writer.flush().unwrap();

        let mut reader = OvBinReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        assert_eq!(a, a_);

        let mut reader = OvBinReader::new(std::io::Cursor::new(&buf));
        let partial_a: PartialA = reader.read().unwrap();
        assert_eq!(a.a, partial_a.a);
        assert_eq!(a.c, partial_a.c);
    }
}
