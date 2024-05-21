use crate::format::{AtomOptionValue, ObjectScheme};

pub trait OvScheme {
    fn object_scheme() -> ObjectScheme;
}

pub trait OvSerialize {
    fn serialize(&self, values: &mut Vec<AtomOptionValue>);
}

pub trait OvDeserialize: Sized {
    fn deserialize(values: &mut &[AtomOptionValue]) -> Option<Self>;
}

#[cfg(test)]
mod tests {
    use crate::format::{AtomOptionType, AtomScheme, AtomType, AtomValue, FieldScheme, ValueType};

    use super::*;

    #[test]
    fn test_serde() {
        #[derive(Debug, PartialEq)]
        struct A {
            a: u16,
            b: B,
            c: Option<f64>,
        }
        impl OvScheme for A {
            fn object_scheme() -> ObjectScheme {
                ObjectScheme {
                    fields: vec![
                        FieldScheme {
                            name: "a".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::U64,
                                nullable: false,
                            }),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Object(<B as OvScheme>::object_scheme()),
                        },
                        FieldScheme {
                            name: "c".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::F64,
                                nullable: true,
                            }),
                        },
                    ],
                }
            }
        }
        impl OvSerialize for A {
            #[allow(clippy::redundant_closure)]
            fn serialize(&self, values: &mut Vec<AtomOptionValue>) {
                values.push(AtomOptionValue::Solid(AtomValue::U64(self.a as _)));
                OvSerialize::serialize(&self.b, values);
                values.push(AtomOptionValue::Option(self.c.map(|x| AtomValue::F64(x))));
            }
        }
        impl OvDeserialize for A {
            fn deserialize(values: &mut &[AtomOptionValue]) -> Option<Self> {
                Some(Self {
                    a: {
                        let value = values.first()?.atom_value()?.u64()? as _;
                        *values = &values[1..];
                        value
                    },
                    b: B::deserialize(values)?,
                    c: {
                        let value = match values.first()?.atom_value() {
                            Some(x) => Some(x.f64()? as _),
                            None => None,
                        };
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
        impl OvScheme for B {
            fn object_scheme() -> ObjectScheme {
                ObjectScheme {
                    fields: vec![
                        FieldScheme {
                            name: "a".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::Bytes,
                                nullable: false,
                            }),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::I64,
                                nullable: false,
                            }),
                        },
                        FieldScheme {
                            name: "c".to_string(),
                            value: ValueType::Atom(AtomOptionType {
                                value: AtomType::Bytes,
                                nullable: false,
                            }),
                        },
                    ],
                }
            }
        }
        impl OvSerialize for B {
            fn serialize(&self, values: &mut Vec<AtomOptionValue>) {
                values.push(AtomOptionValue::Solid(AtomValue::Bytes(self.a.clone())));
                values.push(AtomOptionValue::Solid(AtomValue::I64(self.b)));
                values.push(AtomOptionValue::Solid(AtomValue::Bytes(
                    self.c.as_bytes().to_owned(),
                )));
            }
        }
        impl OvDeserialize for B {
            fn deserialize(values: &mut &[AtomOptionValue]) -> Option<Self> {
                Some(Self {
                    a: {
                        let value = values.first()?.atom_value()?.bytes()?.to_owned();
                        *values = &values[1..];
                        value
                    },
                    b: {
                        let value = values.first()?.atom_value()?.i64()?;
                        *values = &values[1..];
                        value
                    },
                    c: {
                        let value =
                            String::from_utf8(values.first()?.atom_value()?.bytes()?.to_owned())
                                .ok()?;
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
            c: Some(3.),
        };

        let scheme = A::object_scheme();
        assert_eq!(
            scheme,
            ObjectScheme {
                fields: vec![
                    FieldScheme {
                        name: "a".to_owned(),
                        value: ValueType::Atom(AtomOptionType {
                            value: AtomType::U64,
                            nullable: false,
                        }),
                    },
                    FieldScheme {
                        name: "b".to_owned(),
                        value: ValueType::Object(ObjectScheme {
                            fields: vec![
                                FieldScheme {
                                    name: "a".to_owned(),
                                    value: ValueType::Atom(AtomOptionType {
                                        value: AtomType::Bytes,
                                        nullable: false,
                                    }),
                                },
                                FieldScheme {
                                    name: "b".to_owned(),
                                    value: ValueType::Atom(AtomOptionType {
                                        value: AtomType::I64,
                                        nullable: false,
                                    }),
                                },
                                FieldScheme {
                                    name: "c".to_owned(),
                                    value: ValueType::Atom(AtomOptionType {
                                        value: AtomType::Bytes,
                                        nullable: false,
                                    }),
                                },
                            ]
                        }),
                    },
                    FieldScheme {
                        name: "c".to_owned(),
                        value: ValueType::Atom(AtomOptionType {
                            value: AtomType::F64,
                            nullable: true,
                        }),
                    },
                ]
            }
        );

        assert_eq!(
            scheme.atom_schemes(),
            [
                AtomScheme {
                    name: "a".to_owned(),
                    value: AtomOptionType {
                        value: AtomType::U64,
                        nullable: false,
                    },
                },
                AtomScheme {
                    name: "b.a".to_owned(),
                    value: AtomOptionType {
                        value: AtomType::Bytes,
                        nullable: false,
                    },
                },
                AtomScheme {
                    name: "b.b".to_owned(),
                    value: AtomOptionType {
                        value: AtomType::I64,
                        nullable: false,
                    },
                },
                AtomScheme {
                    name: "b.c".to_owned(),
                    value: AtomOptionType {
                        value: AtomType::Bytes,
                        nullable: false,
                    },
                },
                AtomScheme {
                    name: "c".to_owned(),
                    value: AtomOptionType {
                        value: AtomType::F64,
                        nullable: true,
                    },
                },
            ]
        );

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
    }
}
