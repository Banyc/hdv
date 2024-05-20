use crate::format::{AtomValue, ObjectScheme};

pub trait OvSerialize {
    fn object_scheme(&self) -> ObjectScheme;
    fn serialize(&self, values: &mut Vec<AtomValue>);
}

pub trait OvDeserialize: Sized {
    fn deserialize(values: &mut &[AtomValue]) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use crate::format::{AtomScheme, AtomType, FieldScheme, ValueType};

    use super::*;

    #[test]
    fn test_serde() {
        #[derive(Debug, PartialEq)]
        struct A {
            a: u16,
            b: B,
            c: f64,
        }
        impl OvSerialize for A {
            fn object_scheme(&self) -> ObjectScheme {
                ObjectScheme {
                    fields: vec![
                        FieldScheme {
                            name: "a".to_string(),
                            value: ValueType::Atom(AtomType::U64),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Object(OvSerialize::object_scheme(&self.b)),
                        },
                        FieldScheme {
                            name: "c".to_string(),
                            value: ValueType::Atom(AtomType::F64),
                        },
                    ],
                }
            }

            fn serialize(&self, values: &mut Vec<AtomValue>) {
                values.push(AtomValue::U64(self.a as _));
                OvSerialize::serialize(&self.b, values);
                values.push(AtomValue::F64(self.c));
            }
        }
        impl OvDeserialize for A {
            fn deserialize(values: &mut &[AtomValue]) -> Option<Self> {
                Some(Self {
                    a: {
                        let value = values.first()?.u64()? as _;
                        *values = &values[1..];
                        value
                    },
                    b: B::deserialize(values)?,
                    c: {
                        let value = values.first()?.f64()?;
                        *values = &values[1..];
                        value
                    },
                })
            }
        }

        #[derive(Debug, PartialEq)]
        struct B {
            a: Vec<u8>,
            b: i64,
            c: String,
        }
        impl OvSerialize for B {
            fn object_scheme(&self) -> ObjectScheme {
                ObjectScheme {
                    fields: vec![
                        FieldScheme {
                            name: "a".to_string(),
                            value: ValueType::Atom(AtomType::Bytes),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Atom(AtomType::I64),
                        },
                        FieldScheme {
                            name: "c".to_string(),
                            value: ValueType::Atom(AtomType::Bytes),
                        },
                    ],
                }
            }

            fn serialize(&self, values: &mut Vec<AtomValue>) {
                values.push(AtomValue::Bytes(self.a.clone()));
                values.push(AtomValue::I64(self.b));
                values.push(AtomValue::Bytes(self.c.as_bytes().to_owned()));
            }
        }
        impl OvDeserialize for B {
            fn deserialize(values: &mut &[AtomValue]) -> Option<Self> {
                Some(Self {
                    a: {
                        let value = values.first()?.bytes()?.to_owned();
                        *values = &values[1..];
                        value
                    },
                    b: {
                        let value = values.first()?.i64()?;
                        *values = &values[1..];
                        value
                    },
                    c: {
                        let value = String::from_utf8(values.first()?.bytes()?.to_owned()).ok()?;
                        *values = &values[1..];
                        value
                    },
                })
            }
        }

        let a = A {
            a: 1,
            b: B {
                a: b"hello".to_vec(),
                b: 2,
                c: "world".to_owned(),
            },
            c: 3.,
        };

        let scheme = a.object_scheme();
        assert_eq!(
            scheme,
            ObjectScheme {
                fields: vec![
                    FieldScheme {
                        name: "a".to_owned(),
                        value: ValueType::Atom(AtomType::U64),
                    },
                    FieldScheme {
                        name: "b".to_owned(),
                        value: ValueType::Object(ObjectScheme {
                            fields: vec![
                                FieldScheme {
                                    name: "a".to_owned(),
                                    value: ValueType::Atom(AtomType::Bytes),
                                },
                                FieldScheme {
                                    name: "b".to_owned(),
                                    value: ValueType::Atom(AtomType::I64),
                                },
                                FieldScheme {
                                    name: "c".to_owned(),
                                    value: ValueType::Atom(AtomType::Bytes),
                                },
                            ]
                        }),
                    },
                    FieldScheme {
                        name: "c".to_owned(),
                        value: ValueType::Atom(AtomType::F64),
                    },
                ]
            }
        );

        dbg!(scheme.atom_schemes());
        assert_eq!(
            scheme.atom_schemes(),
            [
                AtomScheme {
                    name: "a".to_owned(),
                    value: AtomType::U64,
                },
                AtomScheme {
                    name: "b.a".to_owned(),
                    value: AtomType::Bytes,
                },
                AtomScheme {
                    name: "b.b".to_owned(),
                    value: AtomType::I64,
                },
                AtomScheme {
                    name: "b.c".to_owned(),
                    value: AtomType::Bytes,
                },
                AtomScheme {
                    name: "c".to_owned(),
                    value: AtomType::F64,
                },
            ]
        );

        let mut values = vec![];
        a.serialize(&mut values);
        assert_eq!(
            values,
            [
                AtomValue::U64(1),
                AtomValue::Bytes(b"hello".to_vec()),
                AtomValue::I64(2),
                AtomValue::Bytes(b"world".to_vec()),
                AtomValue::F64(3.0),
            ]
        );

        let b = A::deserialize(&mut &*values).unwrap();
        assert_eq!(a, b);
    }
}
