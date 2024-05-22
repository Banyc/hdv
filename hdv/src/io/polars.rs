use polars::prelude::NamedFrom;

use crate::format::{AtomScheme, AtomType, ValueRow};

use super::{bin::HdvBinRawReader, text::HdvTextRawReader};

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

    Ok(hdv_polars_read(rows, header))
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

    Ok(hdv_polars_read(rows, header))
}

fn hdv_polars_read(rows: Vec<ValueRow>, header: &[AtomScheme]) -> polars::frame::DataFrame {
    let mut series_array = vec![];
    for (i, column_scheme) in header.iter().enumerate() {
        let mut column = vec![];
        for row in &rows {
            let cell = row.atoms()[i].clone();
            column.push(cell);
        }
        let series = match column_scheme.value {
            AtomType::String => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.string().cloned().unwrap()))
                    .collect::<Vec<Option<String>>>();
                polars::series::Series::new(&column_scheme.name, column)
            }
            AtomType::Bytes => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.bytes().cloned().unwrap()))
                    .collect::<Vec<Option<Vec<u8>>>>();
                polars::series::Series::new(&column_scheme.name, column)
            }
            AtomType::U64 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.u64().unwrap()))
                    .collect::<Vec<Option<u64>>>();
                polars::series::Series::new(&column_scheme.name, column)
            }
            AtomType::I64 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.i64().unwrap()))
                    .collect::<Vec<Option<i64>>>();
                polars::series::Series::new(&column_scheme.name, column)
            }
            AtomType::F32 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.f32().unwrap()))
                    .collect::<Vec<Option<f32>>>();
                polars::series::Series::new(&column_scheme.name, column)
            }
            AtomType::F64 => {
                let column = column
                    .into_iter()
                    .map(|x| x.map(|x| x.f64().unwrap()))
                    .collect::<Vec<Option<f64>>>();
                polars::series::Series::new(&column_scheme.name, column)
            }
        };
        series_array.push(series);
    }
    polars::frame::DataFrame::new(series_array).unwrap()
}
