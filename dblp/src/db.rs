//! All database-related items are defined here.
//!
//! That includes all SQL queries.

use std::{borrow::Borrow, io::Write, str::FromStr};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, ToSql};

use crate::dataset::db_items::{DblpRecord, PersonRecord, PublicationRecord, SEPARATOR};

// type DbConnectionPool = Pool<SqliteConnectionManager>;
type DbConnection = PooledConnection<SqliteConnectionManager>;

/// Checks if the database contains the necessary tables, and that they have stuff in them.
pub fn check_database(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT COUNT(name) from persons;")?;
    let _ = stmt.query(())?;

    let mut stmt = conn.prepare("SELECT COUNT(record) from publications;")?;
    let _ = stmt.query(())?;
    // match (num_names, num_pubs) {
    //     (0, 0) => Err(rusqlite::Error::QueryReturnedNoRows),
    //     _ => Ok(()),
    // }

    Ok(())
}

/// Initializes the database tables and drops all indexes.
pub fn create_tables(conn: &DbConnection) -> rusqlite::Result<()> {
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

fn create_all_indexes(conn: &DbConnection) -> rusqlite::Result<()> {
    // this is the only guaranteed unique column in the database
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_profile ON persons(profile)",
        (),
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_year ON publications(year)",
        (),
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_authors ON publications(authors)",
        (),
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_citations ON publications(citations)",
        (),
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_author_name ON persons(name)",
        (),
    )?;

    Ok(())
}

/// Drops all indexes in the database.
fn drop_all_indexes(conn: &DbConnection) -> rusqlite::Result<()> {
    // drop all indexes
    conn.execute("DROP INDEX IF EXISTS idx_profile", ())?;
    conn.execute("DROP INDEX IF EXISTS idx_year", ())?;
    conn.execute("DROP INDEX IF EXISTS idx_authors", ())?;
    conn.execute("DROP INDEX IF EXISTS idx_citations", ())?;
    conn.execute("DROP INDEX IF EXISTS idx_author_name", ())?;

    Ok(())
}

// only valid if all rows are returned! (SELECT * FROM ...)
impl<'a, R: Borrow<rusqlite::Row<'a>>> From<R> for DblpRecord {
    fn from(value: R) -> Self {
        let row: &rusqlite::Row = value.borrow();

        Self {
            record: PublicationRecord::from_str(&row.get::<usize, String>(1).unwrap()).unwrap(),
            key: row.get(2).unwrap(),
            mdate: row.get(3).ok(),
            publtype: row.get(4).ok(),
            year: row.get(5).ok(),
            authors: row.get(6).ok(),
            citations: row.get(7).ok(),
            publisher: row.get(8).ok(),
            school: row.get(9).ok(),
        }
    }
}

// only valid if all rows are returned! (SELECT * FROM ...)
impl<'a, R: Borrow<rusqlite::Row<'a>>> From<R> for PersonRecord {
    fn from(value: R) -> Self {
        let row = value.borrow();

        Self {
            name: row.get(1).unwrap(),
            profile: row.get(2).unwrap(),
            aliases: row.get(3).unwrap(),
        }
    }
}

/// Drops the tables in the database.
pub fn clear_tables(conn: &DbConnection) -> rusqlite::Result<()> {
    // let c = conn.get().unwrap();

    conn.execute("DROP TABLE IF EXISTS persons", ())?;
    conn.execute("DROP TABLE IF EXISTS publications", ())?;

    drop_all_indexes(&conn)?;
    Ok(())
}

/// Inserts the given records into the database.
pub fn dump_into_database(
    conn: &mut DbConnection,
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
pub fn chunked_deserialize_insert(conn: &mut DbConnection, xml_str: &str) -> rusqlite::Result<()> {
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
    println!("creating index...");
    create_all_indexes(conn)?;

    Ok(())
}

/// Raw query into the publications table, given a set of constraints.
pub fn raw_publications_query(
    conn: &Connection,
    constraints: String,
) -> rusqlite::Result<Vec<DblpRecord>> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM publications {};", constraints))?;

    let rows = stmt.query_map((), |r| Ok(DblpRecord::from(r)))?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Raw query into persons table, given a set of constraints.
pub fn raw_persons_query(
    conn: &Connection,
    constraints: String,
) -> rusqlite::Result<Vec<PersonRecord>> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM persons {};", constraints))?;

    let rows = stmt.query_map((), |r| Ok(PersonRecord::from(r)))?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Search for all records from a specific author
pub fn query_author_publications(
    conn: &Connection,
    author: String,
    max_year: Option<u32>,
    limit: Option<u32>,
) -> rusqlite::Result<Vec<DblpRecord>> {
    let q_author = format!("%{}{}{}%", SEPARATOR, author, SEPARATOR);

    let mut box_q_params: Vec<Box<dyn ToSql>> = vec![Box::new(q_author)];

    let mut q_string = format!("SELECT * FROM publications WHERE authors LIKE ? ");

    if let Some(year) = max_year {
        q_string.push_str("AND year <= ? ");
        box_q_params.push(Box::new(year))
    }

    if let Some(l) = limit {
        q_string.push_str("LIMIT ?");
        box_q_params.push(Box::new(l))
    }

    // convert to Vec<&dyn ToSql>
    let q_params: Vec<&dyn ToSql> = box_q_params.iter().map(|b| b.borrow()).collect::<Vec<_>>();

    let mut stmt = conn.prepare(&q_string)?;

    let rows = stmt.query_map(q_params.as_slice(), |r| Ok(DblpRecord::from(r)))?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Query the database for a specific author.
///
/// If there is no match from author name, a search through aliases is performed.
pub fn query_author(
    conn: &Connection,
    author: String,
    limit: Option<u32>,
) -> rusqlite::Result<Vec<PersonRecord>> {
    let mod_author = capitalize_wildcard(&author);
    let mut box_q_params: Vec<Box<dyn ToSql>> = vec![Box::new(&mod_author)];

    let mut q_string = format!("SELECT * FROM persons WHERE name LIKE ? ");

    if let Some(l) = limit {
        q_string.push_str("LIMIT ?");
        box_q_params.push(Box::new(l))
    }

    // convert to Vec<&dyn ToSql>
    let q_params: Vec<&dyn ToSql> = box_q_params.iter().map(|b| b.borrow()).collect::<Vec<_>>();
    let mut stmt = conn.prepare(&q_string)?;
    let rows = stmt.query_map(q_params.as_slice(), |r| Ok(PersonRecord::from(r)))?;

    // if matches found, return
    let initial_results = rows.filter_map(|r| r.ok()).collect::<Vec<_>>();
    match initial_results.len() {
        0 => (),
        _ => return Ok(initial_results),
    }

    // search thru aliases if no exact match found
    let mut q_string = format!("SELECT * FROM persons WHERE aliases LIKE ? ");
    let mut box_q_params: Vec<Box<dyn ToSql>> = vec![Box::new(format!("%{}%", author))];

    if let Some(l) = limit {
        q_string.push_str("LIMIT ?");
        box_q_params.push(Box::new(l))
    }

    // convert to Vec<&dyn ToSql>
    let q_params: Vec<&dyn ToSql> = box_q_params.iter().map(|b| b.borrow()).collect::<Vec<_>>();
    let mut stmt = conn.prepare(&q_string)?;
    let rows = stmt.query_map(q_params.as_slice(), |r| Ok(PersonRecord::from(r)))?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Query the database for a specific publication
pub fn query_publication(
    conn: &Connection,
    key: String,
    limit: Option<u32>,
) -> rusqlite::Result<Vec<DblpRecord>> {
    let mut box_q_params: Vec<Box<dyn ToSql>> = vec![Box::new(key)];

    let mut q_string = format!("SELECT * FROM persons WHERE aliases LIKE ? ");

    if let Some(l) = limit {
        q_string.push_str("LIMIT ?");
        box_q_params.push(Box::new(l))
    }

    // convert to Vec<&dyn ToSql>
    let q_params: Vec<&dyn ToSql> = box_q_params.iter().map(|b| b.borrow()).collect::<Vec<_>>();

    let mut stmt = conn.prepare(&q_string)?;

    let rows = stmt.query_map(q_params.as_slice(), |r| Ok(DblpRecord::from(r)))?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Capitalize the first letter of a name and insert the '%' wildcard in spaces.
fn capitalize_wildcard(input: &str) -> String {
    input
        .split_whitespace()
        .map(|n| {
            let mut chars = n.chars();
            match chars.next() {
                Some(f) => f.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("%")
}

#[cfg(test)]
mod tests {

    use crate::dataset::{strip_references, xml_items::RawDblp};

    use super::*;

    #[test]
    fn test_init_database() {
        // let conn = rusqlite::Connection::open("./test.sqlite").unwrap();
        let conn = r2d2_sqlite::SqliteConnectionManager::memory();
        let pool = r2d2::Pool::new(conn).unwrap();
        let c = pool.get().unwrap();

        create_tables(&c).unwrap();
    }

    #[test]
    fn test_push_to_database() {
        let conn = r2d2_sqlite::SqliteConnectionManager::memory();
        let pool = r2d2::Pool::new(conn).unwrap();
        let mut c = pool.get().unwrap();

        // let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();
        let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();

        let filtered = strip_references(&contents);

        let dblp: RawDblp = quick_xml::de::from_str(&filtered).unwrap();

        let (publications, persons): (Vec<DblpRecord>, Vec<PersonRecord>) = dblp.into();

        create_tables(&c).unwrap();
        dump_into_database(&mut c, &publications, &persons).unwrap();
    }

    #[test]
    fn test_capitalize_wildcard() {
        let input = "john doe";
        let expected = "John%Doe";
        assert_eq!(capitalize_wildcard(input), expected);
    }
}
