use option::extract_type_from_option;
use ov::format::AtomType;

mod option;

#[proc_macro_derive(OvSerde)]
pub fn serde(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let serde = syn::parse_macro_input!(input as Serde);

    let impl_scheme = impl_scheme(&serde);
    let impl_serialize = impl_serialize(&serde);
    let impl_deserialize = impl_deserialize(&serde);
    let impl_all = quote::quote! {
        #impl_scheme
        #impl_serialize
        #impl_deserialize
    };
    impl_all.into()
}

#[allow(non_snake_case)]
fn impl_scheme(serde: &Serde) -> proc_macro2::TokenStream {
    let OvScheme = ov_scheme_type();
    let mut field_schemes = vec![];
    for field in &serde.fields {
        let double_quoted_field_name = field.ident.to_string();
        let FieldScheme = field_scheme_type();
        let ValueType = value_type_type();
        let value = match &field.ty {
            FieldType::Object(x) => {
                quote::quote! {
                    #ValueType::Object(<#x as #OvScheme>::object_scheme())
                }
            }
            FieldType::Atom(x) => {
                let AtomTypeArm = x.atom_type_arm();
                let AtomType = atom_type_type();
                quote::quote! {
                    #ValueType::Atom(#AtomType::#AtomTypeArm)
                }
            }
        };
        field_schemes.push(quote::quote! {
            #FieldScheme {
                name: #double_quoted_field_name.to_string(),
                value: #value,
            },
        });
    }
    let Name = &serde.name;
    let ObjectScheme = object_scheme_type();
    quote::quote! {
        impl #OvScheme for #Name {
            fn object_scheme() -> #ObjectScheme {
                #ObjectScheme {
                    fields: vec![
                        #( #field_schemes )*
                    ],
                }
            }
        }
    }
}
#[allow(non_snake_case)]
fn impl_serialize(serde: &Serde) -> proc_macro2::TokenStream {
    let OvSerialize = ov_serialize_type();
    let mut write_values = vec![];
    let AtomValue = atom_value_type();
    for field in &serde.fields {
        let field_name = &field.ident;
        let write_value = match &field.ty {
            FieldType::Object(Name) => {
                if field.nullable {
                    quote::quote! {
                        if let Some(x) = self.#field_name.as_ref() {
                            #OvSerialize::serialize(x, values);
                        } else {
                            <#Name as OvSerialize>::fill_nulls(values);
                        }
                    }
                } else {
                    quote::quote! { #OvSerialize::serialize(&self.#field_name, values); }
                }
            }
            FieldType::Atom(atom_type) => {
                let AtomTypeArm = atom_type.atom_type_arm();
                let convert_type = |x: proc_macro2::TokenStream| match &atom_type {
                    HighLevelAtomType::Compatible(AtomType::String)
                    | HighLevelAtomType::Compatible(AtomType::Bytes) => {
                        quote::quote! { #x.clone() }
                    }
                    HighLevelAtomType::Compatible(_) => quote::quote! { #x as _ },
                };
                let atom_option_value = if field.nullable {
                    let convert_type = convert_type(quote::quote! { x });
                    match &atom_type {
                        HighLevelAtomType::Compatible(AtomType::String)
                        | HighLevelAtomType::Compatible(AtomType::Bytes) => {
                            quote::quote! { self.#field_name.as_ref().map(|x| #AtomValue::#AtomTypeArm(#convert_type)) }
                        }
                        HighLevelAtomType::Compatible(_) => {
                            quote::quote! { self.#field_name.map(|x| #AtomValue::#AtomTypeArm(#convert_type)) }
                        }
                    }
                } else {
                    let convert_type = convert_type(quote::quote! { self.#field_name });
                    quote::quote! { Some(#AtomValue::#AtomTypeArm(#convert_type)) }
                };
                quote::quote! { values.push(#atom_option_value); }
            }
        };
        write_values.push(write_value);
    }

    let mut fill_nulls = vec![];
    for field in &serde.fields {
        let fill_null = match &field.ty {
            FieldType::Object(Name) => {
                quote::quote! { <#Name as OvSerialize>::fill_nulls(values); }
            }
            FieldType::Atom(_) => {
                quote::quote! { values.push(None); }
            }
        };
        fill_nulls.push(fill_null);
    }

    let Name = &serde.name;
    quote::quote! {
        impl #OvSerialize for #Name {
            fn serialize(&self, values: &mut Vec<Option<#AtomValue>>) {
                #( #write_values )*
            }

            fn fill_nulls(values: &mut Vec<Option<#AtomValue>>) {
                #( #fill_nulls )*
            }
        }
    }
}
#[allow(non_snake_case)]
fn impl_deserialize(serde: &Serde) -> proc_macro2::TokenStream {
    let OvDeserialize = ov_deserialize_type();
    let mut fetch_values = vec![];
    for field in &serde.fields {
        let field_name = &field.ident;
        let fetch_value = match &field.ty {
            FieldType::Object(Name) => {
                quote::quote! { let #field_name = <#Name as #OvDeserialize>::deserialize(__values); }
            }
            FieldType::Atom(_) => {
                quote::quote! {
                    let #field_name = {
                        let value = __values.first()?.as_ref();
                        *__values = &__values[1..];
                        value
                    };
                }
            }
        };
        fetch_values.push(fetch_value);
    }
    let mut fields = vec![];
    for field in &serde.fields {
        let field_name = &field.ident;
        let field_value = match &field.ty {
            FieldType::Object(_) => {
                if field.nullable {
                    quote::quote! { #field_name }
                } else {
                    quote::quote! { #field_name? }
                }
            }
            FieldType::Atom(x) => {
                let atom_type_get = x.atom_type_get();
                let convert_type = |atom_value: proc_macro2::TokenStream| match &x {
                    HighLevelAtomType::Compatible(AtomType::String)
                    | HighLevelAtomType::Compatible(AtomType::Bytes) => {
                        quote::quote! { #atom_value.#atom_type_get.unwrap().to_owned() }
                    }
                    HighLevelAtomType::Compatible(_) => {
                        quote::quote! { #atom_value.#atom_type_get.unwrap() as _ }
                    }
                };
                if field.nullable {
                    let convert_type = convert_type(quote::quote! { x });
                    quote::quote! {
                        #field_name.map(|x| #convert_type)
                    }
                } else {
                    let convert_type = convert_type(quote::quote! { #field_name? });
                    quote::quote! { #convert_type }
                }
            }
        };
        fields.push(quote::quote! { #field_name: #field_value, });
    }
    let Name = &serde.name;
    let AtomValue = atom_value_type();
    quote::quote! {
        impl #OvDeserialize for #Name {
            fn deserialize(__values: &mut &[Option<#AtomValue>]) -> Option<Self> {
                #( #fetch_values )*
                Some(Self {
                    #( #fields )*
                })
            }
        }
    }
}

fn ov_scheme_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::serde::OvScheme
    }
}
fn ov_serialize_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::serde::OvSerialize
    }
}
fn ov_deserialize_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::serde::OvDeserialize
    }
}
fn object_scheme_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::serde::ObjectScheme
    }
}
fn field_scheme_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::serde::FieldScheme
    }
}
fn value_type_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::serde::ValueType
    }
}
fn atom_type_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::format::AtomType
    }
}
fn atom_value_type() -> proc_macro2::TokenStream {
    quote::quote! {
        ov::format::AtomValue
    }
}

struct Serde {
    pub name: syn::Ident,
    pub fields: Vec<Field>,
}
impl syn::parse::Parse for Serde {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let stmt: syn::Stmt = input.parse()?;
        let item = match stmt {
            syn::Stmt::Item(x) => x,
            _ => {
                return Err(syn::Error::new(
                    input.span(),
                    format!(
                        "`stmt` expected `Stmt::Item`, found `{}`",
                        quote::quote! { #stmt }
                    ),
                ));
            }
        };
        let structure = match item {
            syn::Item::Struct(x) => x,
            _ => {
                return Err(syn::Error::new(
                    input.span(),
                    format!(
                        "`item` expected `Item::Struct`, found `{}`",
                        quote::quote! { #item }
                    ),
                ));
            }
        };
        let name = structure.ident;
        let mut fields = vec![];
        for field in &structure.fields {
            let ident = &field.ident;
            let Some(ident) = ident else {
                return Err(syn::Error::new(
                    input.span(),
                    format!(
                        "`ident` expected `Some`, found `{}`",
                        quote::quote! { #ident }
                    ),
                ));
            };
            let (ty, nullable) = field_type(&field.ty)?;
            fields.push(Field {
                ident: ident.clone(),
                ty,
                nullable,
            })
        }
        Ok(Self { name, fields })
    }
}

fn field_type(ty: &syn::Type) -> syn::Result<(FieldType, bool)> {
    let (ty, nullable) = match extract_type_from_option(ty) {
        Some(ty) => (ty, true),
        None => (ty, false),
    };
    let str = quote::quote! { #ty }.to_string();
    let field_type = match str.as_str() {
        "u8" | "u16" | "u32" | "u64" => {
            FieldType::Atom(HighLevelAtomType::Compatible(AtomType::U64))
        }
        "i8" | "i16" | "i32" | "i64" => {
            FieldType::Atom(HighLevelAtomType::Compatible(AtomType::I64))
        }
        "f32" => FieldType::Atom(HighLevelAtomType::Compatible(AtomType::F32)),
        "f64" => FieldType::Atom(HighLevelAtomType::Compatible(AtomType::F64)),
        "Vec < u8 >" => FieldType::Atom(HighLevelAtomType::Compatible(AtomType::Bytes)),
        "String" => FieldType::Atom(HighLevelAtomType::Compatible(AtomType::String)),
        _ => FieldType::Object(ty.clone()),
    };
    Ok((field_type, nullable))
}

struct Field {
    pub ident: syn::Ident,
    pub ty: FieldType,
    pub nullable: bool,
}
enum FieldType {
    Object(syn::Type),
    Atom(HighLevelAtomType),
}
enum HighLevelAtomType {
    Compatible(AtomType),
}
impl HighLevelAtomType {
    pub fn atom_type_arm(&self) -> proc_macro2::TokenStream {
        match self {
            HighLevelAtomType::Compatible(AtomType::String) => {
                quote::quote! { String }
            }
            HighLevelAtomType::Compatible(AtomType::Bytes) => {
                quote::quote! { Bytes }
            }
            HighLevelAtomType::Compatible(AtomType::F32) => {
                quote::quote! { F32 }
            }
            HighLevelAtomType::Compatible(AtomType::F64) => {
                quote::quote! { F64 }
            }
            HighLevelAtomType::Compatible(AtomType::I64) => {
                quote::quote! { I64 }
            }
            HighLevelAtomType::Compatible(AtomType::U64) => {
                quote::quote! { U64 }
            }
        }
    }

    pub fn atom_type_get(&self) -> proc_macro2::TokenStream {
        match self {
            HighLevelAtomType::Compatible(AtomType::String) => {
                quote::quote! { string() }
            }
            HighLevelAtomType::Compatible(AtomType::Bytes) => {
                quote::quote! { bytes() }
            }
            HighLevelAtomType::Compatible(AtomType::F32) => {
                quote::quote! { f32() }
            }
            HighLevelAtomType::Compatible(AtomType::F64) => {
                quote::quote! { f64() }
            }
            HighLevelAtomType::Compatible(AtomType::I64) => {
                quote::quote! { i64() }
            }
            HighLevelAtomType::Compatible(AtomType::U64) => {
                quote::quote! { u64() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn show_ast() {
        let src = r#"
            #[derive(Debug, PartialEq)]
            struct A {
                a: u16,
                b: B,
                c: f64,
                d: Vec<u8>,
                e: String,
                f: Option<i8>,
            }
        "#;
        let res = syn::parse_str::<syn::Stmt>(src).unwrap();
        println!("{:#?}", res);
    }
}
