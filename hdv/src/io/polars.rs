use polars::prelude::NamedFrom;

use crate::{
    format::{AtomScheme, AtomType, AtomValue, ValueRow},
    io::bin::HdvBinRawWriter,
};

use super::{
    bin::HdvBinRawReader,
    text::{HdvTextRawReader, HdvTextRawWriter, HdvTextWriterOptions},
};

pub fn hdv_bin_polars_write<W>(write: W, df: &polars::frame::DataFrame) -> std::io::Result<()>
where
    W: std::io::Write,
{
    let (rows, header) = hdv_polars_write(df).ok_or(std::io::ErrorKind::InvalidInput)?;
    let mut writer = HdvBinRawWriter::new(write, header);
    for row in &rows {
        writer.write(row)?;
    }
    Ok(())
}
pub fn hdv_text_polars_write<W>(
    write: W,
    df: &polars::frame::DataFrame,
    options: HdvTextWriterOptions,
) -> std::io::Result<()>
where
    W: std::io::Write,
{
    let (rows, header) = hdv_polars_write(df).ok_or(std::io::ErrorKind::InvalidInput)?;
    let mut writer = HdvTextRawWriter::new(write, header, options);
    for row in &rows {
        writer.write(row)?;
    }
    Ok(())
}

pub fn hdv_bin_polars_read<R>(read: R) -> std::io::Result<polars::frame::DataFrame>
where
    R: std::io::Read,
{
    let mut reader = HdvBinRawReader::new(read);
    let mut rows = vec![];
    loop {
        let res = reader.read();
        let row = match res {
            Ok(x) => x,
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                _ => return Err(e),
            },
        };
        rows.push(row);
    }
    let header = match reader.header() {
        Some(x) => x,
        None => return Ok(polars::frame::DataFrame::empty()),
    };

    Ok(hdv_polars_read(rows.iter(), header))
}
pub fn hdv_text_polars_read<R>(read: R) -> std::io::Result<polars::frame::DataFrame>
where
    R: std::io::BufRead,
{
    let mut reader = HdvTextRawReader::new(read);
    let mut rows = vec![];
    loop {
        let res = reader.read();
        let row = match res {
            Ok(x) => x,
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                _ => return Err(e),
            },
        };
        rows.push(row);
    }
    let header = match reader.header() {
        Some(x) => x,
        None => return Ok(polars::frame::DataFrame::empty()),
    };

    Ok(hdv_polars_read(rows.iter(), header))
}

fn hdv_polars_write(df: &polars::frame::DataFrame) -> Option<(Vec<ValueRow>, Vec<AtomScheme>)> {
    let series_array = df.get_columns();
    let mut header = vec![];
    for series in series_array {
        let atom_type = match series._dtype() {
            polars::datatypes::DataType::Boolean => AtomType::Bool,
            polars::datatypes::DataType::UInt8
            | polars::datatypes::DataType::UInt16
            | polars::datatypes::DataType::UInt32
            | polars::datatypes::DataType::UInt64 => AtomType::U64,
            polars::datatypes::DataType::Int8
            | polars::datatypes::DataType::Int16
            | polars::datatypes::DataType::Int32
            | polars::datatypes::DataType::Int64 => AtomType::I64,
            polars::datatypes::DataType::Float32 => AtomType::F32,
            polars::datatypes::DataType::Float64 => AtomType::F64,
            polars::datatypes::DataType::String => AtomType::String,
            polars::datatypes::DataType::Binary => AtomType::Bytes,
            _ => return None,
        };
        header.push(AtomScheme {
            name: series.name().to_string(),
            r#type: atom_type,
        });
    }
    let mut columns = vec![];
    for (scheme, series) in header.iter().zip(series_array.iter()) {
        let column: Vec<Option<AtomValue>> = match scheme.r#type {
            AtomType::String => series
                .str()
                .unwrap()
                .iter()
                .map(|x| x.map(|x| x.into()).map(AtomValue::String))
                .collect(),
            AtomType::Bytes => series
                .binary()
                .unwrap()
                .iter()
                .map(|x| x.map(|x| x.into()).map(AtomValue::Bytes))
                .collect(),
            AtomType::U64 => series
                .cast(&polars::datatypes::DataType::UInt64)
                .unwrap()
                .u64()
                .unwrap()
                .iter()
                .map(|x| x.map(AtomValue::U64))
                .collect(),
            AtomType::I64 => series
                .cast(&polars::datatypes::DataType::Int64)
                .unwrap()
                .i64()
                .unwrap()
                .iter()
                .map(|x| x.map(AtomValue::I64))
                .collect(),
            AtomType::F32 => series
                .cast(&polars::datatypes::DataType::Float32)
                .unwrap()
                .f32()
                .unwrap()
                .iter()
                .map(|x| x.map(AtomValue::F32))
                .collect(),
            AtomType::F64 => series
                .cast(&polars::datatypes::DataType::Float64)
                .unwrap()
                .f64()
                .unwrap()
                .iter()
                .map(|x| x.map(AtomValue::F64))
                .collect(),
            AtomType::Bool => series
                .cast(&polars::datatypes::DataType::Boolean)
                .unwrap()
                .bool()
                .unwrap()
                .iter()
                .map(|x| x.map(AtomValue::Bool))
                .collect(),
        };
        columns.push(column);
    }
    let mut rows = vec![];
    let len = columns.first()?.len();
    for i in 0..len {
        let mut atoms = vec![];
        for column in &columns {
            atoms.push(column[i].clone());
        }
        let row = ValueRow::new(atoms);
        rows.push(row);
    }
    Some((rows, header))
}
fn hdv_polars_read<'a>(
    rows: impl Iterator<Item = &'a ValueRow> + Clone,
    header: &[AtomScheme],
) -> polars::frame::DataFrame {
    let mut series_array = vec![];
    for (i, column_scheme) in header.iter().enumerate() {
        let mut column = vec![];
        for row in rows.clone() {
            let cell = row.atoms()[i].clone();
            column.push(cell);
        }
        let series = match column_scheme.r#type {
            AtomType::String => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.string().map(|x| x.to_string()).unwrap()))
                    .collect::<Vec<Option<String>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
            AtomType::Bytes => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.bytes().map(|x| x.to_vec()).unwrap()))
                    .collect::<Vec<Option<Vec<u8>>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
            AtomType::U64 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.u64().unwrap()))
                    .collect::<Vec<Option<u64>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
            AtomType::I64 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.i64().unwrap()))
                    .collect::<Vec<Option<i64>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
            AtomType::F32 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.f32().unwrap()))
                    .collect::<Vec<Option<f32>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
            AtomType::F64 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.f64().unwrap()))
                    .collect::<Vec<Option<f64>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
            AtomType::Bool => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.bool().unwrap()))
                    .collect::<Vec<Option<bool>>>();
                polars::series::Series::new(column_scheme.name.clone().into(), column)
            }
        };
        series_array.push(series);
    }
    polars::frame::DataFrame::new(series_array).unwrap()
}
