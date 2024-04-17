//! All database-related items are defined here.
//!
//! That includes all (most) SQL queries.

use std::{borrow::Borrow, collections::HashSet, io::Write, str::FromStr, sync::mpsc};

use r2d2::{Pool, PooledConnection};
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
            id: row.get(0).unwrap(),
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
            id: row.get(0).unwrap(),
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
    conn: &DbConnection,
    author: String,
    exact: bool,
    limit: Option<u32>,
) -> rusqlite::Result<Vec<PersonRecord>> {
    if exact {
        return query_author_exact(conn, &author);
    }

    // some author names have a serial number at the end, like this:
    // - "John Doe 0001"
    // so we query for that as well

    let mod_author = capitalize_wildcard(&author);
    let mod_author_serial = format!("{} ____", mod_author);
    let mut box_q_params: Vec<Box<dyn ToSql>> =
        vec![Box::new(&mod_author), Box::new(mod_author_serial)];

    let mut q_string = format!("SELECT * FROM persons WHERE name LIKE ? OR name LIKE ?");

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

/// Run the query with an exact match
pub fn query_author_exact(
    conn: &DbConnection,
    author: &str,
) -> rusqlite::Result<Vec<PersonRecord>> {
    let mut stmt = conn.prepare("SELECT * FROM persons WHERE name = ?")?;

    let rows = stmt.query_map(&[&format!("::{}::", author)], |r| Ok(PersonRecord::from(r)))?;
    Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
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

/// Create an in-memory subset of the database, where the publications
/// are filtered by coauthors and year range.
///
/// Returns the connection pool to the subset database.
#[allow(unused)]
pub fn create_subset_database(
    conn: &DbConnection,
    authors: &[PersonRecord],
    start: u32,
    end: u32,
) -> rusqlite::Result<Pool<SqliteConnectionManager>> {
    let mgr = SqliteConnectionManager::file("subset.sqlite"); // temp file
    let pool = Pool::new(mgr).unwrap();

    let s_conn = pool.get().unwrap();

    clear_tables(&s_conn)?;
    create_tables(&s_conn)?;

    // author task
    let (a_tx, a_rx) = mpsc::channel::<PersonRecord>();
    let p_h1 = pool.clone();
    let h1 = std::thread::spawn(move || {
        let mut conn = p_h1.get().unwrap();

        let transaction = conn.transaction().expect("failed to create transaction");
        let mut stmt = transaction
            .prepare(
                "INSERT INTO persons
            (name, profile, aliases)
            VALUES (?, ?, ?)",
            )
            .expect("failed to create prepare statement");

        while let Ok(data) = a_rx.recv() {
            stmt.execute((
                data.name.to_owned(),
                data.profile.to_owned(),
                data.aliases.to_owned(),
            ))
            .expect("failed to insert data");
        }

        drop(stmt);
        transaction.finish().expect("failed to ocmmit transaction");
    });

    // publication task
    let (p_tx, p_rx) = mpsc::channel::<DblpRecord>();
    let p_h2 = pool.clone();
    let h2 = std::thread::spawn(move || {
        let mut conn = p_h2.get().unwrap();

        let transaction = conn.transaction().expect("failed to create transaction");

        let mut stmt = transaction
            .prepare(
                "INSERT INTO publications
            (record, key, mdate, publtype, year, authors, citations, publisher, school)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .expect("failed to create prepare statement");

        while let Ok(data) = p_rx.recv() {
            stmt.execute((
                data.record.to_string(),
                data.key.to_owned(),
                data.mdate.to_owned(),
                data.publtype.to_owned(),
                data.year.to_owned(),
                data.authors.to_owned(),
                data.citations.to_owned(),
                data.publisher.to_owned(),
                data.school.to_owned(),
            ))
            .expect("failed to insert data");
        }

        drop(stmt);
        transaction.finish().expect("failed to commit transaction");
    });

    let mut insert_set = HashSet::<u32>::new();

    for author in authors {
        let x = query_author_publications(&conn, author.name.clone(), Some(end), None)?;

        let insert = x
            .into_iter()
            .filter_map(|record| match insert_set.contains(&record.id) {
                true => None,
                false => {
                    insert_set.insert(record.id);
                    Some(record)
                }
            });

        for record in insert {
            p_tx.send(record).expect("failed to send data");
        }
    }

    for a in authors {
        a_tx.send(a.clone()).expect("failed to send data");
    }

    drop(a_tx);
    drop(p_tx);
    h1.join();
    h2.join();

    Ok(pool)
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

    use crate::{
        dataset::{strip_references, xml_items::RawDblp},
        get_init_conn_pool, DB_PATH,
    };

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

    /// Test if rusqlite can copy data from one db to another.
    /// Does not work
    #[test]
    fn test_database_copy() -> rusqlite::Result<()> {
        let mgr = SqliteConnectionManager::file("subset.sqlite");
        let pool = Pool::new(mgr).unwrap();

        // clear_tables(conn)
        create_tables(&pool.get().unwrap())?;

        // init og database
        DB_PATH.get_or_init(|| "../dblp.sqlite".to_string());

        let conn = get_init_conn_pool();

        let res = conn.query_row("SELECT * FROM persons LIMIT 10", (), |r| {
            r.get::<usize, String>(1)
        })?;
        // assert_eq!(res, 10);
        println!("{}", res);

        let res = conn.execute(
            "ATTACH DATABASE 'subset.sqlite' AS subset_db;
        INSERT INTO subset_db.persons SELECT * FROM persons",
            (),
        )?;
        // let res = conn.execute("INSERT INTO subset_db.persons SELECT * FROM persons", ())?;

        // create_tables(conn)

        Ok(())
    }
}
