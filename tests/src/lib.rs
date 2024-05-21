#[cfg(test)]
mod tests {
    use ov::{
        format::{AtomOptionValue, AtomValue},
        io::{OvReader, OvWriter},
        serde::{OvDeserialize, OvSerialize},
    };
    use ov_derive::OvSerde;

    #[test]
    fn test_derive() {
        #[derive(Debug, OvSerde, PartialEq)]
        pub struct A {
            a: u16,
            b: B,
            c: Option<f64>,
        }
        #[derive(Debug, OvSerde, PartialEq)]
        struct B {
            a: Vec<u8>,
            b: i64,
            c: String,
        }

        let a = A {
            a: 1,
            b: B {
                a: b"hello".to_vec(),
                b: 2,
                c: "world".to_owned(),
            },
            c: Some(3.),
        };

        let mut values = vec![];
        a.serialize(&mut values);
        assert_eq!(
            values,
            [
                AtomOptionValue::Solid(AtomValue::U64(1)),
                AtomOptionValue::Solid(AtomValue::Bytes(b"hello".to_vec())),
                AtomOptionValue::Solid(AtomValue::I64(2)),
                AtomOptionValue::Solid(AtomValue::Bytes(b"world".to_vec())),
                AtomOptionValue::Option(Some(AtomValue::F64(3.0))),
            ]
        );

        let b = A::deserialize(&mut &*values).unwrap();
        assert_eq!(a, b);

        let mut buf = vec![];
        let mut writer = OvWriter::new(&mut buf);
        writer.write(&a).unwrap();
        writer.flush().unwrap();

        let mut reader = OvReader::new(std::io::Cursor::new(&buf));
        let a_: A = reader.read().unwrap();
        assert_eq!(a, a_);
    }
}
