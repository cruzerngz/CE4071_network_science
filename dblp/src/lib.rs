//! dblp is a library for parsing and querying the DBLP XML file.

mod dataset;
mod db;

use std::{
    cell::OnceCell,
    fs,
    io::Read,
    sync::{Arc, Once, OnceLock, RwLock},
};

use dataset::xml_items::RawDblp;
use db::{create_tables, dump_into_database};
use pyo3::{exceptions::PyTypeError, prelude::*};

const DB_DEFAULT_PATH: &str = "dblp.sqlite";
const GZIP_DEFAULT_PATH: &str = "dblp.xml.gz";
const XML_DEFAULT_PATH: &str = "dblp.xml";

static DB_PATH: OnceLock<String> = OnceLock::new();

/// Initialize the DBLP database from a local file.
///
/// The file can be an xml file or a gzipped xml file `*.xml`, `*.xml.gz`.
///
/// If no file is specified, the default gzipped file `dblp.xml.gz` is used.
#[pyfunction]
pub fn init_from_xml(path: Option<String>) -> PyResult<()> {
    let actual_path = path.as_deref().unwrap_or(GZIP_DEFAULT_PATH);

    let xml_file = fs::read(actual_path).map_err(PyTypeError::new_err)?;

    let xml_data = match actual_path.ends_with(".gz") {
        true => {
            log::info!("Decompressing the gzipped file");

            let mut xml_bytes = Vec::new();
            let mut decoder = flate2::read::GzDecoder::new(xml_file.as_slice());
            decoder
                .read_to_end(&mut xml_bytes)
                .map_err(PyTypeError::new_err)?;

            let raw_xml_str = std::str::from_utf8(&xml_bytes).map_err(PyTypeError::new_err)?;
            let filtered_xml_str = dataset::strip_references(raw_xml_str);

            // fs::write(XML_DEFAULT_PATH, &filtered_xml_str).map_err(PyTypeError::new_err)?;

            filtered_xml_str
        }
        false => {
            log::info!("Reading the xml file");
            let raw_xml = std::str::from_utf8(&xml_file).map_err(PyTypeError::new_err)?;
            let filt_xml = dataset::strip_references(raw_xml);

            // filter out the references
            // fs::write(actual_path, &filt_xml).map_err(PyTypeError::new_err)?;

            filt_xml
        }
    };

    let raw_dataset: RawDblp =
        quick_xml::de::from_str(&xml_data).map_err(|e| PyTypeError::new_err(e.to_string()))?;

    let (publications, persons): (Vec<_>, Vec<_>) = raw_dataset.into();

    let mut conn = rusqlite::Connection::open(DB_DEFAULT_PATH)
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    create_tables(&conn).map_err(|e| PyTypeError::new_err(e.to_string()))?;
    dump_into_database(&mut conn, &publications, &persons)
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    // initialize the db path
    DB_PATH.get_or_init(|| DB_DEFAULT_PATH.to_string());

    Ok(())
}

/// Initialize the DBLP database from a sqlite file previously parsed by `dblp`.
///
/// If no path is provided, the default path `dblp.sqlite` is used.
#[pyfunction]
pub fn init_from_sqlite(path: Option<String>) -> PyResult<()> {
    let conn = match &path {
        Some(path) => rusqlite::Connection::open(path),
        None => rusqlite::Connection::open(DB_DEFAULT_PATH),
    }
    .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::check_database(&conn).map_err(|e| PyTypeError::new_err(e.to_string()))?;

    // initialize the db path
    DB_PATH.get_or_init(|| path.unwrap_or(DB_DEFAULT_PATH.to_string()));

    Ok(())
}

/// Query the DBLP database
#[pyfunction]
pub fn hello_world() -> PyResult<()> {
    println!("Hwllo world!");

    Ok(())
}

/// dblp is a library for parsing and querying the DBLP XML file.
#[pymodule]
fn dblp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_world, m)?)?;
    m.add_function(wrap_pyfunction!(init_from_xml, m)?)?;
    m.add_function(wrap_pyfunction!(init_from_sqlite, m)?)?;
    Ok(())
}
