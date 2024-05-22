# `ov`

Object values.

CSV but can be parsed in a multi-layer `struct`.

## Usage

### Import dependencies

```rust
use ov::{
    format::AtomValue,
    io::{
        bin::{OvBinReader, OvBinWriter},
        text::{OvTextReader, OvTextWriter, OvTextWriterOptions},
    },
    serde::{OvDeserialize, OvSerialize},
};
use ov_derive::OvSerde;
```

### Write and read data in binary format

```rust
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

let mut buf = vec![];
let mut writer = OvBinWriter::new(&mut buf);
writer.write(&a).unwrap();
writer.flush().unwrap();

let mut reader = OvBinReader::new(std::io::Cursor::new(&buf));
let a_: A = reader.read().unwrap();
assert_eq!(a, a_);

#[derive(Debug, OvSerde, PartialEq)]
pub struct PartialA {
    c: Option<f64>,
    a: u16,
}

let mut reader = OvBinReader::new(std::io::Cursor::new(&buf));
let partial_a: PartialA = reader.read().unwrap();
assert_eq!(a.a, partial_a.a);
assert_eq!(a.c, partial_a.c);
```

### Write and read data in text format

Currently the text format does not accept:

- bytes (`Vec<u8>`);
- strings containing any of the chars `,`, `"`, and `\n` or starting with whitespace characters.

```rust
#[derive(Debug, OvSerde, PartialEq)]
pub struct A {
    a: u16,
    b: Option<B>,
    c: Option<f64>,
    d: B,
}
#[derive(Debug, OvSerde, PartialEq)]
struct B {
    b: i64,
    c: String,
}

let a = A {
    a: 1,
    b: None,
    c: Some(3.),
    d: B {
        b: 2,
        c: "world".to_owned(),
    },
};

let mut buf = vec![];
let options = OvTextWriterOptions {
    is_csv_header: false,
};
let mut writer = OvTextWriter::new(&mut buf, options);
writer.write(&a).unwrap();
writer.flush().unwrap();

let mut reader = OvTextReader::new(std::io::Cursor::new(&buf));
let a_: A = reader.read().unwrap();
assert_eq!(a, a_);

#[derive(Debug, OvSerde, PartialEq)]
pub struct PartialA {
    c: Option<f64>,
    a: u16,
}

let mut reader = OvTextReader::new(std::io::Cursor::new(&buf));
let partial_a: PartialA = reader.read().unwrap();
assert_eq!(a.a, partial_a.a);
assert_eq!(a.c, partial_a.c);
```
