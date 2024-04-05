//! dblp is a library for parsing and querying the DBLP XML file.

mod dataset;
mod db;

use std::{fs, io::Read, sync::OnceLock};

use dataset::db_items::{DblpRecord, PersonRecord};
use pyo3::{exceptions::PyTypeError, prelude::*};

const DB_DEFAULT_PATH: &str = "dblp.sqlite";
const GZIP_DEFAULT_PATH: &str = "dblp.xml.gz";

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
            println!("Decompressing the gzipped file");

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
            println!("Reading xml file");
            let raw_xml = std::str::from_utf8(&xml_file).map_err(PyTypeError::new_err)?;
            let filt_xml = dataset::strip_references(raw_xml);

            // filter out the references
            // fs::write(actual_path, &filt_xml).map_err(PyTypeError::new_err)?;

            filt_xml
        }
    };

    drop(xml_file);

    let mut conn = rusqlite::Connection::open(DB_DEFAULT_PATH)
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::clear_tables(&conn).map_err(|e| PyTypeError::new_err(e.to_string()))?;
    db::create_tables(&conn).map_err(|e| PyTypeError::new_err(e.to_string()))?;

    // initialize the db path
    DB_PATH.get_or_init(|| DB_DEFAULT_PATH.to_string());

    println!("writing chunks to: {}", DB_PATH.get().unwrap());
    db::chunked_deserialize_insert(&mut conn, &xml_data)
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

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

/// Perform a raw query on the persons table.
#[pyfunction]
pub fn query_persons_table(constraints: String) -> PyResult<Vec<PersonRecord>> {
    let conn = rusqlite::Connection::open(DB_PATH.get().unwrap())
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::raw_persons_query(&conn, constraints).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Perform a raw query on the publications table.
#[pyfunction]
pub fn query_publications_table(constraints: String) -> PyResult<Vec<DblpRecord>> {
    let conn = rusqlite::Connection::open(DB_PATH.get().unwrap())
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::raw_publications_query(&conn, constraints).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Search for an author in the database.
#[pyfunction]
pub fn query_person(name: String, limit: Option<u32>) -> PyResult<Vec<PersonRecord>> {
    let conn = rusqlite::Connection::open(DB_PATH.get().unwrap())
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::query_author(&conn, name, limit).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Search for a publication in the database.
#[pyfunction]
pub fn query_publication(title: String, limit: Option<u32>) -> PyResult<Vec<DblpRecord>> {
    let conn = rusqlite::Connection::open(DB_PATH.get().unwrap())
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::query_publication(&conn, title, limit).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Search for all publications from a specific author.
///
/// The `limit` parameter can be used to limit the number of results.
/// The `max_year` parameter can be used to limit the results up to a specific year.
#[pyfunction]
pub fn query_person_publications(
    name: String,
    max_year: Option<u32>,
    limit: Option<u32>,
) -> PyResult<Vec<DblpRecord>> {
    let conn = rusqlite::Connection::open(DB_PATH.get().unwrap())
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    db::query_author_publications(&conn, name, max_year, limit)
        .map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// dblp is a library for parsing and querying the DBLP XML file.
#[pymodule]
fn dblp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_world, m)?)?;
    m.add_function(wrap_pyfunction!(init_from_xml, m)?)?;
    m.add_function(wrap_pyfunction!(init_from_sqlite, m)?)?;
    m.add_function(wrap_pyfunction!(query_persons_table, m)?)?;
    m.add_function(wrap_pyfunction!(query_publications_table, m)?)?;
    m.add_function(wrap_pyfunction!(query_person, m)?)?;
    m.add_function(wrap_pyfunction!(query_person_publications, m)?)?;
    m.add_class::<dataset::db_items::DblpRecord>()?;
    m.add_class::<dataset::db_items::PersonRecord>()?;
    m.add_class::<dataset::db_items::PublicationRecord>()?;
    Ok(())
}
