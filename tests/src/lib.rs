#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hdv::{
        format::AtomValue,
        io::{
            bin::{HdvBinReader, HdvBinWriter},
            text::{HdvTextReader, HdvTextWriter, HdvTextWriterOptions},
        },
        serde::{HdvDeserialize, HdvSerialize},
    };
    use hdv_derive::HdvSerde;

    #[test]
    fn test_derive_bin() {
        #[derive(Debug, HdvSerde, PartialEq)]
        pub struct A {
            a: u16,
            b: Option<B>,
            c: Option<f64>,
            d: B,
        }
        #[derive(Debug, HdvSerde, PartialEq)]
        struct B {
            a: Arc<[u8]>,
            b: i64,
            c: Arc<str>,
            d: Option<Arc<[u8]>>,
        }

        let a = A {
            a: 1,
            b: None,
            c: Some(3.),
            d: B {
                a: b"hello".as_ref().into(),
                b: 2,
                c: "world".into(),
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
                Some(AtomValue::Bytes(b"hello".as_ref().into())),
                Some(AtomValue::I64(2)),
                Some(AtomValue::String("world".into())),
                None,
            ]
        );

        let b = A::deserialize(&mut &*values).unwrap();
        assert_eq!(a, b);

        let mut buf = vec![];
        let mut writer = HdvBinWriter::new(&mut buf);
        writer.write(&a).unwrap();
        writer.flush().unwrap();

        let mut reader = HdvBinReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        assert_eq!(a, a_);

        #[derive(Debug, HdvSerde, PartialEq)]
        pub struct PartialA {
            c: Option<f64>,
            a: u16,
        }

        let mut reader = HdvBinReader::new(std::io::Cursor::new(&buf));
        let partial_a: PartialA = reader.read().unwrap();
        assert_eq!(a.a, partial_a.a);
        assert_eq!(a.c, partial_a.c);
    }

    #[test]
    fn test_derive_text() {
        #[derive(Debug, HdvSerde, PartialEq)]
        pub struct A {
            a: u16,
            b: Option<B>,
            c: Option<f64>,
            d: B,
        }
        #[derive(Debug, HdvSerde, PartialEq)]
        struct B {
            b: i64,
            c: Arc<str>,
        }

        let a = A {
            a: 1,
            b: None,
            c: Some(3.),
            d: B {
                b: 2,
                c: "world".into(),
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
                Some(AtomValue::F64(3.0)),
                Some(AtomValue::I64(2)),
                Some(AtomValue::String("world".into())),
            ]
        );

        let b = A::deserialize(&mut &*values).unwrap();
        assert_eq!(a, b);

        let mut buf = vec![];
        let options = HdvTextWriterOptions {
            is_csv_header: false,
        };
        let mut writer = HdvTextWriter::new(&mut buf, options);
        writer.write(&a).unwrap();
        writer.flush().unwrap();

        let mut reader = HdvTextReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        assert_eq!(a, a_);

        #[derive(Debug, HdvSerde, PartialEq)]
        pub struct PartialA {
            c: Option<f64>,
            a: u16,
        }

        let mut reader = HdvTextReader::new(std::io::Cursor::new(&buf));
        let partial_a: PartialA = reader.read().unwrap();
        assert_eq!(a.a, partial_a.a);
        assert_eq!(a.c, partial_a.c);
    }
}
