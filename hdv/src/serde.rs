use crate::format::{AtomScheme, AtomType, AtomValue};

pub trait HdvScheme {
    fn object_scheme() -> ObjectScheme;
}

pub trait HdvSerialize {
    fn serialize(&self, values: &mut Vec<Option<AtomValue>>);
    fn fill_nulls(values: &mut Vec<Option<AtomValue>>);
}

pub trait HdvDeserialize: Sized {
    fn deserialize(values: &mut &[Option<AtomValue>]) -> Option<Self>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct ObjectScheme {
    pub fields: Vec<FieldScheme>,
}
impl ObjectScheme {
    pub fn atom_schemes(&self) -> Vec<AtomScheme> {
        let mut atoms = vec![];
        for field in &self.fields {
            atoms.extend(field.atom_schemes().into_iter());
        }
        atoms
    }

    pub fn atom_types(&self, types: &mut Vec<AtomType>) {
        for field in &self.fields {
            field.atom_types(types);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FieldScheme {
    pub name: String,
    pub value: ValueType,
}
impl FieldScheme {
    pub fn atom_schemes(&self) -> Vec<AtomScheme> {
        let post_atoms = match &self.value {
            ValueType::Atom(x) => {
                return vec![AtomScheme {
                    name: self.name.clone(),
                    r#type: *x,
                }]
            }
            ValueType::Object(object) => object.atom_schemes(),
        };
        let mut atoms = vec![];
        for post_atom in &post_atoms {
            let name = format!("{}.{}", self.name, post_atom.name);
            atoms.push(AtomScheme {
                name,
                r#type: post_atom.r#type,
            });
        }
        atoms
    }

    pub fn atom_types(&self, types: &mut Vec<AtomType>) {
        match &self.value {
            ValueType::Atom(x) => types.push(*x),
            ValueType::Object(object) => object.atom_types(types),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ValueType {
    Atom(AtomType),
    Object(ObjectScheme),
}

#[cfg(test)]
mod tests {
    use crate::format::{AtomScheme, AtomType, AtomValue};

    use super::*;

    #[test]
    fn test_serde() {
        #[derive(Debug, PartialEq)]
        pub struct A {
            a: u16,
            b: Option<B>,
            c: Option<f64>,
            d: B,
        }
        impl HdvScheme for A {
            fn object_scheme() -> ObjectScheme {
                ObjectScheme {
                    fields: vec![
                        FieldScheme {
                            name: "a".to_string(),
                            value: ValueType::Atom(AtomType::U64),
                        },
                        FieldScheme {
                            name: "b".to_string(),
                            value: ValueType::Object(<B as HdvScheme>::object_scheme()),
                        },
                        FieldScheme {
                            name: "c".to_string(),
                            value: ValueType::Atom(AtomType::F64),
                        },
                        FieldScheme {
                            name: "d".to_string(),
                            value: ValueType::Object(<B as HdvScheme>::object_scheme()),
                        },
                    ],
                }
            }
        }
        impl HdvSerialize for A {
            #[allow(clippy::redundant_closure)]
            fn serialize(&self, values: &mut Vec<Option<AtomValue>>) {
                values.push(Some(AtomValue::U64(self.a as _)));
                if let Some(x) = self.b.as_ref() {
                    HdvSerialize::serialize(x, values);
                } else {
                    <B as HdvSerialize>::fill_nulls(values);
                }
                values.push(self.c.map(|x| AtomValue::F64(x as _)));
                HdvSerialize::serialize(&self.d, values);
            }

            fn fill_nulls(values: &mut Vec<Option<AtomValue>>) {
                values.push(None);
                <B as HdvSerialize>::fill_nulls(values);
                values.push(None);
                <B as HdvSerialize>::fill_nulls(values);
            }
        }
        impl HdvDeserialize for A {
            #[allow(clippy::redundant_field_names)]
            fn deserialize(__values: &mut &[Option<AtomValue>]) -> Option<Self> {
                let a = {
                    let value = __values.first()?.as_ref();
                    *__values = &__values[1..];
                    value
                };
                let b = <B as HdvDeserialize>::deserialize(__values);
                let c = {
                    let value = __values.first()?.as_ref();
                    *__values = &__values[1..];
                    value
                };
                let d = <B as HdvDeserialize>::deserialize(__values);
                Some(Self {
                    a: a?.u64().unwrap() as _,
                    b: b,
                    c: c.map(|x| x.f64().unwrap() as _),
                    d: d?,
                })
            }
        }

        #[derive(Debug, PartialEq)]
        struct B {
            a: Vec<u8>,
            b: i64,
            c: String,
            d: Option<Vec<u8>>,
        }
        impl HdvScheme for B {
            fn object_scheme() -> ObjectScheme {
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
                            value: ValueType::Atom(AtomType::String),
                        },
                        FieldScheme {
                            name: "d".to_string(),
                            value: ValueType::Atom(AtomType::Bytes),
                        },
                    ],
                }
            }
        }
        impl HdvSerialize for B {
            fn serialize(&self, values: &mut Vec<Option<AtomValue>>) {
                values.push(Some(AtomValue::Bytes(self.a.clone())));
                values.push(Some(AtomValue::I64(self.b)));
                values.push(Some(AtomValue::String(self.c.clone())));
                values.push(self.d.as_ref().map(|x| AtomValue::Bytes(x.clone())));
            }

            fn fill_nulls(values: &mut Vec<Option<AtomValue>>) {
                values.push(None);
                values.push(None);
                values.push(None);
                values.push(None);
            }
        }
        impl HdvDeserialize for B {
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
                let c = {
                    let value = values.first()?.as_ref();
                    *values = &values[1..];
                    value
                };
                let d = {
                    let value = values.first()?.as_ref();
                    *values = &values[1..];
                    value
                };
                Some(Self {
                    a: a?.bytes().unwrap().to_owned(),
                    b: b?.i64().unwrap() as _,
                    c: c?.string().unwrap().to_owned(),
                    d: d.map(|x| x.bytes().unwrap().to_owned()),
                })
            }
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

        let scheme = A::object_scheme();
        assert_eq!(
            scheme,
            ObjectScheme {
                fields: vec![
                    FieldScheme {
                        name: "a".to_owned(),
                        value: ValueType::Atom(AtomType::U64,),
                    },
                    FieldScheme {
                        name: "b".to_owned(),
                        value: ValueType::Object(ObjectScheme {
                            fields: vec![
                                FieldScheme {
                                    name: "a".to_owned(),
                                    value: ValueType::Atom(AtomType::Bytes,),
                                },
                                FieldScheme {
                                    name: "b".to_owned(),
                                    value: ValueType::Atom(AtomType::I64,),
                                },
                                FieldScheme {
                                    name: "c".to_owned(),
                                    value: ValueType::Atom(AtomType::String,),
                                },
                                FieldScheme {
                                    name: "d".to_owned(),
                                    value: ValueType::Atom(AtomType::Bytes,),
                                },
                            ]
                        }),
                    },
                    FieldScheme {
                        name: "c".to_owned(),
                        value: ValueType::Atom(AtomType::F64,),
                    },
                    FieldScheme {
                        name: "d".to_owned(),
                        value: ValueType::Object(ObjectScheme {
                            fields: vec![
                                FieldScheme {
                                    name: "a".to_owned(),
                                    value: ValueType::Atom(AtomType::Bytes,),
                                },
                                FieldScheme {
                                    name: "b".to_owned(),
                                    value: ValueType::Atom(AtomType::I64,),
                                },
                                FieldScheme {
                                    name: "c".to_owned(),
                                    value: ValueType::Atom(AtomType::String,),
                                },
                                FieldScheme {
                                    name: "d".to_owned(),
                                    value: ValueType::Atom(AtomType::Bytes,),
                                },
                            ]
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
                    r#type: AtomType::U64,
                },
                AtomScheme {
                    name: "b.a".to_owned(),
                    r#type: AtomType::Bytes,
                },
                AtomScheme {
                    name: "b.b".to_owned(),
                    r#type: AtomType::I64,
                },
                AtomScheme {
                    name: "b.c".to_owned(),
                    r#type: AtomType::String,
                },
                AtomScheme {
                    name: "b.d".to_owned(),
                    r#type: AtomType::Bytes,
                },
                AtomScheme {
                    name: "c".to_owned(),
                    r#type: AtomType::F64,
                },
                AtomScheme {
                    name: "d.a".to_owned(),
                    r#type: AtomType::Bytes,
                },
                AtomScheme {
                    name: "d.b".to_owned(),
                    r#type: AtomType::I64,
                },
                AtomScheme {
                    name: "d.c".to_owned(),
                    r#type: AtomType::String,
                },
                AtomScheme {
                    name: "d.d".to_owned(),
                    r#type: AtomType::Bytes,
                },
            ]
        );

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
    }
}
