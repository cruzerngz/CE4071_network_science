//! dblp is a library for parsing and querying the DBLP XML file.

mod dataset;
mod db;

use std::{
    collections::HashSet,
    fs,
    io::Read,
    sync::{Arc, Mutex, OnceLock},
};

use chrono::Datelike;
use dataset::db_items::{DblpRecord, PersonRecord, PersonTemporalRelation};
use pyo3::{exceptions::PyTypeError, prelude::*};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use scheduled_thread_pool::ScheduledThreadPool;

use crate::db::create_subset_database;

const DB_DEFAULT_PATH: &str = "dblp.sqlite";
const XML_GZ_PATH: &str = "dblp.xml.gz";
const XML_PATH: &str = "dblp.xml";

static DB_PATH: OnceLock<String> = OnceLock::new();

/// Connection pool to the database
pub static DB_CONN_POOL: OnceLock<Pool<SqliteConnectionManager>> = OnceLock::new();

/// Initialize the connection pool and get a connection.
fn get_init_conn_pool() -> PooledConnection<SqliteConnectionManager> {
    match DB_CONN_POOL.get() {
        Some(p) => p.get().expect("database path not initialized"),
        None => {
            DB_CONN_POOL.get_or_init(|| {
                let manager = SqliteConnectionManager::file(
                    DB_PATH.get().expect("database path not initialized"),
                );
                // thread pool over all available cores
                r2d2::Pool::builder()
                    .max_size(50)
                    .thread_pool(Arc::new(ScheduledThreadPool::new(num_cpus::get() * 2)))
                    .build(manager)
                    .expect("failed to create connection pool")
            });

            DB_CONN_POOL
                .get()
                .expect("connection pool should be initialized")
                .get()
                .unwrap()
        }
    }
}

/// Initialize the DBLP database from a local file.
///
/// The file can be an xml file or a gzipped xml file `*.xml`, `*.xml.gz`.
///
/// If no file is specified, the default gzipped file `dblp.xml.gz` is used.
#[pyfunction]
pub fn init_from_xml(path: Option<String>) -> PyResult<()> {
    let actual_path = match path.as_deref() {
        Some(p) => p,
        None => match (fs::metadata(XML_GZ_PATH), fs::metadata(XML_PATH)) {
            (_, Ok(_)) => XML_PATH,
            (Ok(_), Err(_)) => XML_GZ_PATH,
            (Err(_), Err(_)) => return Err(PyTypeError::new_err("No XML file found")),
        },
    };

    let xml_file = fs::read(actual_path).map_err(PyTypeError::new_err)?;

    let xml_data = match actual_path.ends_with(".gz") {
        true => {
            println!("Reading gzip file");

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

            filt_xml
        }
    };

    drop(xml_file);

    let mut conn = get_init_conn_pool();
    // rusqlite::Connection::open(DB_DEFAULT_PATH)
    // .map_err(|e| PyTypeError::new_err(e.to_string()))?;

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
    DB_PATH.get_or_init(|| path.unwrap_or(DB_DEFAULT_PATH.to_string()));

    let conn = get_init_conn_pool();

    db::check_database(&conn).map_err(|e| PyTypeError::new_err(e.to_string()))?;

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
    let conn = get_init_conn_pool();

    db::raw_persons_query(&conn, constraints).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Perform a raw query on the publications table.
#[pyfunction]
pub fn query_publications_table(constraints: String) -> PyResult<Vec<DblpRecord>> {
    let conn = get_init_conn_pool();

    db::raw_publications_query(&conn, constraints).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Search for an author in the database.
///
/// If looking for an exact match, set `exact` to `true`.
#[pyfunction]
#[pyo3(signature = (name, exact=false, limit=None))]
pub fn query_person(name: String, exact: bool, limit: Option<u32>) -> PyResult<Vec<PersonRecord>> {
    let conn = get_init_conn_pool();
    db::query_author(&conn, name, exact, limit).map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Search for a publication in the database.
#[pyfunction]
pub fn query_publication(title: String, limit: Option<u32>) -> PyResult<Vec<DblpRecord>> {
    let conn = get_init_conn_pool();

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
    let conn = get_init_conn_pool();

    db::query_author_publications(&conn, name, max_year, limit)
        .map_err(|e| PyTypeError::new_err(e.to_string()))
}

/// Transform [PersonRecord] vectors to [PersonTemporalRelation] vectors.
///
/// Specify the start and end year ranges (optional).
/// By default, the starting year is 2000, and the end year is the current year.
///
/// TODO: create an in-memory subset database, then perform the query.
#[pyfunction]
#[pyo3(signature = (persons, year_start=2000, year_end=None, verbose=false))]
pub fn temporal_relation(
    persons: Vec<PersonRecord>,
    year_start: u32,
    year_end: Option<u32>,
    verbose: bool,
) -> Vec<PersonTemporalRelation> {
    // partitioning didnt make a diff it seems
    // let conn = get_init_conn_pool();
    // // create subset db
    // let subset_pool = create_subset_database(
    //     &conn,
    //     &persons,
    //     year_start,
    //     year_end.unwrap_or(chrono::Local::now().year() as u32),
    // )
    // .expect("failed to create subset database");
    // // temp return point
    // return vec![];

    let mut results = vec![];

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let total = persons.len();
    println!();

    let handle = std::thread::spawn(move || {
        let mut count = 0;
        while let Ok(_) = rx.recv() {
            count += 1;
            print!("\rprocessed: {}/{}", count, total);
        }
    });

    let constraints = persons
        .iter()
        .map(|p| p.name.clone())
        .collect::<HashSet<_>>();

    // parallelize in chunks
    for chunk in persons.chunks(2) {
        let res = Mutex::new(vec![
            PersonTemporalRelation {
                author: "".to_string(),
                years: (0, 0),
                coauthor_years: vec![]
            };
            chunk.len()
        ]);

        chunk.par_iter().enumerate().for_each(|(idx, person)| {
            let rel = match person.to_relations(
                year_start,
                year_end.unwrap_or(chrono::Local::now().year() as u32),
                &constraints,
            ) {
                Ok(r) => r,
                Err(e) => {
                    // if verbose {
                    eprintln!("Error: {}", e);
                    // }
                    return;
                }
            };
            tx.send(()).expect("send failed");
            // println!("constructed: {:?}", rel);
            res.lock().unwrap()[idx] = rel;
        });

        results.extend(res.into_inner().unwrap());
    }

    drop(tx);
    handle.join().unwrap();

    results
}

/// Write the temporal relations to a csv file.
#[pyfunction]
pub fn save_temporal_relation(
    relation: Vec<PersonTemporalRelation>,
    target: String,
) -> PyResult<()> {
    if relation.len() == 0 {
        return Ok(());
    }

    let first = relation[0].years;
    if !relation.iter().all(|r| r.years == first) {
        return Err(PyTypeError::new_err(
            "records do not all share the same year range",
        ));
    }

    let mut writer =
        csv::Writer::from_path(&target).map_err(|e| PyTypeError::new_err(e.to_string()))?;

    writer
        .write_record(relation[0].to_csv_headers())
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    for rec in relation.iter() {
        writer
            .write_record(rec.to_csv_row())
            .map_err(|e| PyTypeError::new_err(e.to_string()))?;
    }

    writer
        .flush()
        .map_err(|e| PyTypeError::new_err(e.to_string()))?;

    Ok(())
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
    m.add_function(wrap_pyfunction!(temporal_relation, m)?)?;
    m.add_function(wrap_pyfunction!(save_temporal_relation, m)?)?;
    m.add_class::<dataset::db_items::DblpRecord>()?;
    m.add_class::<dataset::db_items::PersonRecord>()?;
    m.add_class::<dataset::db_items::PublicationRecord>()?;
    m.add_class::<dataset::db_items::PersonTemporalRelation>()?;
    Ok(())
}
