#![allow(unused)]

use core::num;
use std::io::Write;

use pyo3::Python;
use rusqlite::Connection;

use crate::dataset::db_items::{DblpRecord, PersonRecord};

/// Checks if the database contains the necessary tables, and that they have stuff in them.
pub fn check_database(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT COUNT(name) from persons;")?;
    let num_names = stmt.query(())?;

    let mut stmt = conn.prepare("SELECT COUNT(record) from publications;")?;
    let _ = stmt.query(())?;
    // match (num_names, num_pubs) {
    //     (0, 0) => Err(rusqlite::Error::QueryReturnedNoRows),
    //     _ => Ok(()),
    // }

    Ok(())
}

/// Initializes the database tables.
pub fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS persons(
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            profile TEXT NOT NULL,
            aliases TEXT NOT NULL
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS publications(
            id INTEGER PRIMARY KEY,
            record TEXT NOT NULL,
            key TEXT NOT NULL,
            mdate TEXT,
            publtype TEXT,
            year INTEGER,
            authors TEXT,
            citations TEXT,
            publisher TEXT,
            school TEXT
        )",
        (),
    )?;

    Ok(())
}

/// Drops the tables in the database.
pub fn clear_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DROP TABLE persons", ())?;
    conn.execute("DROP TABLE publications", ())?;

    Ok(())
}

/// Inserts the given records into the database.
pub fn dump_into_database(
    conn: &mut Connection,
    records: &[DblpRecord],
    persons: &[PersonRecord],
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;

    let mut stmt = tx.prepare(
        "INSERT INTO publications
    (record, key, mdate, publtype, year, authors, citations, publisher, school)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;

    for publication in records.iter() {
        stmt.execute((
            publication.record.to_string(),
            publication.key.to_owned(),
            publication.mdate.to_owned(),
            publication.publtype.to_owned(),
            publication.year.to_owned(),
            publication.authors.to_owned(),
            publication.citations.to_owned(),
            publication.publisher.to_owned(),
            publication.school.to_owned(),
        ))?;
    }
    drop(stmt);

    let mut stmt = tx.prepare("INSERT INTO persons (name, profile, aliases) VALUES (?, ?, ?)")?;

    for person in persons.iter() {
        stmt.execute((
            person.name.to_owned(),
            person.profile.to_owned(),
            person.aliases.to_owned(),
        ))?;
    }

    drop(stmt);
    tx.commit()?;

    Ok(())
}

/// Deserialize the XML in chunks and insert into the database.
/// The input XML should already be filtered of references.
///
/// Me small computer. Me no ram.
pub fn chunked_deserialize_insert(conn: &mut Connection, xml_str: &str) -> rusqlite::Result<()> {
    const CHUNK_SIZE: usize = 1000;

    // process large num of elements at a time
    let chonker = crate::dataset::ChunkedXmlViewer::from_str(xml_str, CHUNK_SIZE);

    let mut chunk_number = 0;
    println!("");
    for chunk in chonker {
        print!("\rProcessed {} elements", chunk_number * CHUNK_SIZE);
        std::io::stdout().flush().unwrap();
        chunk_number += 1;

        let dblp: crate::dataset::xml_items::RawDblp = quick_xml::de::from_str(&chunk).unwrap();

        let (publications, persons): (Vec<DblpRecord>, Vec<PersonRecord>) = dblp.into();

        dump_into_database(conn, &publications, &persons)?;
    }

    println!();

    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::dataset::{strip_references, xml_items::RawDblp};

    use super::*;

    #[test]
    fn test_init_database() {
        // let conn = rusqlite::Connection::open("./test.sqlite").unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
    }

    #[test]
    fn test_push_to_database() {
        let mut conn = rusqlite::Connection::open("./test.sqlite").unwrap();
        // let mut conn = rusqlite::Connection::open_in_memory().unwrap();

        // let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();
        let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();

        let filtered = strip_references(&contents);

        let dblp: RawDblp = quick_xml::de::from_str(&filtered).unwrap();

        let (publications, persons): (Vec<DblpRecord>, Vec<PersonRecord>) = dblp.into();

        create_tables(&conn).unwrap();
        dump_into_database(&mut conn, &publications, &persons).unwrap();
    }
}
