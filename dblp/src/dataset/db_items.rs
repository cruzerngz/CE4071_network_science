//! Definitions for use with the embedded sqlite database.
//!
//! Basically, the data types defined in this module are filtered versions of
//! the ones in [super::data_items].

use std::{borrow::Borrow, collections::HashSet, fmt::Display, str::FromStr};

use pyo3::{exceptions::PyTypeError, pyclass, pymethods, PyRef, PyRefMut, PyResult};
use serde::{Deserialize, Serialize};

use crate::{db, get_init_conn_pool};

use super::xml_items::*;

/// The separator used to join vectors in the database.
pub const SEPARATOR: &str = "::";

/// A single entry in the database.
/// Each publication type is squashed into the "record" field.
///
/// Vectors are joined with double colons: "::"
#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DblpRecord {
    #[pyo3(get)]
    pub record: PublicationRecord,

    /// The key of the publication record.
    /// Can be a conference, journal, etc.
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub mdate: Option<String>,
    #[pyo3(get)]
    pub publtype: Option<String>,

    #[pyo3(get)]
    pub year: Option<u32>,

    /// Authors are referenced by their profile.
    // #[pyo3(get)]
    pub authors: Option<String>,

    /// Other publications referenced by this publication.
    /// Publications are referenced by their key.
    #[pyo3(get)]
    pub citations: Option<String>,

    /// Publisher of the publication.
    #[pyo3(get)]
    pub publisher: Option<String>,

    #[pyo3(get)]
    pub school: Option<String>,
}

/// Iterable so that [PublicationRecord] can be converted to a python dictionary
#[pyclass]
#[derive(Clone, Debug)]
pub struct DblpRecordIter {
    field: u8,
    inner: DblpRecord,
}

/// The type of publication record in the database.
#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PublicationRecord {
    Article,
    InProceeding,
    Proceeding,
    InCollection,
    Book,
    Collection,
    PhdThesis,
    MastersThesis,
    Data,
}

/// A single person record. Usually the author of a publication.
#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonRecord {
    /// Primary key
    pub id: u32,

    #[pyo3(get)]
    pub name: String,

    /// This is unique across all person records.
    /// Used to distinguish between people.
    #[pyo3(get)]
    pub profile: String,

    /// Other names the person is known by.
    #[pyo3(get)]
    pub aliases: String,
}

/// Iterator for [PersonRecord]
#[pyclass]
#[derive(Clone, Debug)]
pub struct PersonRecordIter {
    field: u8,
    inner: PersonRecord,
}

/// Coauthor relations of a particular author over time
///
/// This isn't a DB item (not a row in table) but is composed of other items in the database.
#[pyclass]
#[derive(Clone, Debug)]
pub struct PersonTemporalRelation {
    pub author: String,

    /// Year start and end range, inclusive, for validation
    pub years: (u32, u32),

    /// Coauthors with this author on a particular year
    /// The index of the vector corresponds to the year, not inclusive of offset in `years`.
    pub coauthor_years: Vec<HashSet<String>>,
}

/// Used in the creation of [PersonTemporalRelation]
struct AssociatedAuthorYear {
    year: u32,
    authors: String,
}

impl<'a, R: Borrow<rusqlite::Row<'a>>> From<R> for AssociatedAuthorYear {
    fn from(value: R) -> Self {
        let row = value.borrow();

        Self {
            year: row.get(0).unwrap(),
            authors: row.get(1).unwrap(),
        }
    }
}

#[pymethods]
impl DblpRecord {
    pub fn __iter__(slf: PyRef<'_, Self>) -> DblpRecordIter {
        DblpRecordIter {
            field: 0,
            inner: slf.to_owned(),
        }
    }

    pub fn __str__(&self) -> String {
        format!("{:?}", self)
    }

    pub fn authors(&self) -> Option<Vec<String>> {
        let authors = self.authors.as_deref()?;

        Some(
            authors
                .trim_matches(SEPARATOR.chars().collect::<Vec<_>>().as_slice())
                .split(SEPARATOR)
                .map(|a| a.to_string())
                .collect(),
        )
    }
}

#[pymethods]
impl PersonRecord {
    pub fn __iter__(slf: PyRef<'_, Self>) -> PersonRecordIter {
        PersonRecordIter {
            field: 0,
            inner: slf.to_owned(),
        }
    }

    pub fn __str__(&self) -> String {
        format!("{:?}", self)
    }

    /// Returns a list of coauthors for the person.
    pub fn coauthors(&self) -> PyResult<Vec<String>> {
        let conn = get_init_conn_pool();

        // optim step: select only the authors field and collect into a set
        let publications =
            db::raw_publications_query(&conn, format!("WHERE authors like '%::{}::%'", self.name))
                .map_err(|e| PyTypeError::new_err(e.to_string()))?;

        // println!("publications found: {}", publications.len());

        let co_auth_names = publications
            .iter()
            .map(|p| {
                p.authors
                    .as_ref()
                    .and_then(|a| Some(a.as_str()))
                    .unwrap_or("")
                    .trim_end_matches(SEPARATOR)
                    .split(SEPARATOR)
                    .map(|name| name)
            })
            .flatten()
            .collect::<HashSet<_>>();

        Ok(co_auth_names
            .iter()
            .filter_map(|n| match n.len() {
                0 => None,
                _ => Some(n.to_string()),
            })
            .collect())
    }
}

impl PersonRecord {
    /// Construct the temporal relations of the person with their coauthors.
    ///
    /// This is the only way to construct a [PersonTemporalRelation].
    ///
    /// TODO: optimize to one query, then do post-processing
    pub(crate) fn to_relations(
        &self,
        start: u32,
        end: u32,
    ) -> rusqlite::Result<PersonTemporalRelation> {
        let mut relation = PersonTemporalRelation {
            author: self.name.to_string(),
            years: (start, end),
            coauthor_years: vec![],
        };
        let conn = get_init_conn_pool();

        let mut stmt = conn.prepare(&format!(
            "
            SELECT publications.year, publications.authors
            FROM persons
            JOIN publications ON publications.authors LIKE '%::' || persons.name  || '::%'
            WHERE publications.year >= ? AND publications.year <= ?
            AND persons.id = ?
            ORDER BY publications.year ASC
        "
        ))?;

        let rows = stmt.query_map(rusqlite::params![start, end, self.id], |r| {
            Ok(AssociatedAuthorYear::from(r))
        })?;

        let a = rows.filter_map(|r| r.ok()).collect::<Vec<_>>();
        let mut co_authors = HashSet::new();

        for yr in start..=end {
            let mut co_auth = HashSet::new();

            for assoc in a.iter().filter(|a| a.year == yr) {
                co_auth.extend(
                    assoc.authors
                        .trim_end_matches(SEPARATOR)
                        .split(SEPARATOR)
                        .map(|c| c.to_string())
                        .collect::<HashSet<_>>(),
                );
            }

            co_authors.extend(co_auth);
            co_authors.remove(&self.name); // remove self from coauthors
            relation.coauthor_years.push(co_authors.clone());
        }

        // inclusive range
        // for _ in start..=end {

        //     let rows = stmt.query_map(rusqlite::params![start, end, self.id], |r| {
        //         r.get::<usize, String>(0)
        //     })?;

        //     let co_auth = rows
        //         .filter_map(|r| match r {
        //             Ok(co) => Some(
        //                 co.split(SEPARATOR)
        //                     .map(|c| c.to_string())
        //                     .collect::<HashSet<_>>(),
        //             ),
        //             Err(_) => None,
        //         })
        //         .flatten()
        //         .filter_map(|n| match n.len() {
        //             0 => None,
        //             _ => Some(n),
        //         })
        //         .collect::<HashSet<_>>();

        //     relation.coauthor_years.push(co_auth);
        // }

        Ok(relation)
    }
}

#[pymethods]
impl DblpRecordIter {
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(String, Option<String>)> {
        match slf.field {
            0 => {
                slf.field += 1;
                Some(("record".to_string(), Some(slf.inner.record.to_string())))
            }
            1 => {
                slf.field += 1;
                Some(("key".to_string(), Some(slf.inner.key.clone())))
            }
            2 => {
                slf.field += 1;
                Some(("mdate".to_string(), slf.inner.mdate.clone()))
            }
            3 => {
                slf.field += 1;
                Some(("publtype".to_string(), slf.inner.publtype.clone()))
            }
            4 => {
                slf.field += 1;
                Some(("year".to_string(), slf.inner.year.map(|y| y.to_string())))
            }
            5 => {
                slf.field += 1;
                Some(("authors".to_string(), slf.inner.authors.clone()))
            }
            6 => {
                slf.field += 1;
                Some(("citations".to_string(), slf.inner.citations.clone()))
            }
            7 => {
                slf.field += 1;
                Some(("publisher".to_string(), slf.inner.publisher.clone()))
            }
            8 => {
                slf.field += 1;
                Some(("school".to_string(), slf.inner.school.clone()))
            }
            _ => None,
        }
    }
}

#[pymethods]
impl PersonRecordIter {
    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(String, Option<String>)> {
        match slf.field {
            0 => {
                slf.field += 1;
                Some(("name".to_string(), Some(slf.inner.name.clone())))
            }
            1 => {
                slf.field += 1;
                Some(("profile".to_string(), Some(slf.inner.profile.clone())))
            }
            2 => {
                slf.field += 1;
                Some(("aliases".to_string(), Some(slf.inner.aliases.clone())))
            }

            _ => None,
        }
    }
}

#[pymethods]
impl PersonTemporalRelation {
    pub fn __str__(&self) -> String {
        format!("{:?}", self)
    }

    /// From internal metadata, generate the csv headers (variable, depending on the year range)
    pub(crate) fn to_csv_headers(&self) -> Vec<String> {
        let mut headers = Vec::new();
        headers.push("author".to_string());

        for year in self.years.0..=self.years.1 {
            headers.push(year.to_string());
        }

        headers
    }

    /// Generate the data for csv row
    pub(crate) fn to_csv_row(&self) -> Vec<String> {
        // incrementally combine the sets from each year
        let mut row = Vec::new();
        row.push(self.author.to_owned());

        // row.extend(self.coauthor_years.iter().map(|c| {
        //     c.iter()
        //         .map(|c| c.to_string())
        //         .collect::<Vec<_>>()
        //         .join(SEPARATOR)
        // }));

        for limit in 1..=self.coauthor_years.len() {
            let mut coauthors = HashSet::new();

            for i in 0..limit {
                coauthors.extend(self.coauthor_years[i].iter().cloned());
            }

            row.push(
                coauthors
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(SEPARATOR),
            );
        }

        row
    }
}

/// Join a vector of strings.
/// The separator is also placed at the start and end of the string.
fn join_string_vec(vec: Vec<String>) -> String {
    match vec.len() {
        0 => String::new(),
        _ => {
            format!("{}{}{}", SEPARATOR, vec.join(SEPARATOR), SEPARATOR)
        }
    }
}

macro_rules! try_into_dblp_record {
    {$from_ty: ty, $rcrd: expr} => {
        impl TryFrom<$from_ty> for DblpRecord {
            type Error = ();

            fn try_from(value: $from_ty) -> Result<Self, Self::Error> {
                Ok(Self {
                    record: $rcrd,
                    key: value.0.key,
                    mdate: value.0.mdate.and_then(|d| Some(d.to_string())),
                    publtype: value.0.publtype,
                    year: value.0.year,
                    authors: {
                        let val = value
                        .0
                        .authors
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>();

                        match val.len() {
                            0 => None,
                            _ => Some(join_string_vec(val))
                        }
                    },
                    // citations have weird shit and need to be filtered out:
                    // - blank strings
                    // - elipses: ...
                    citations: {
                        let val = value.0.citations
                            .into_iter()
                            .filter_map(|c| {
                                if c.len() == 0 {
                                    return None
                                }

                                match c.chars().all(|c| !char::is_alphabetic(c)) {
                                    true => None,
                                    false => Some(c)
                                }
                            })
                            .collect::<Vec<_>>();

                        match val.len() {
                            0 => None,
                            _ => Some(join_string_vec(val))
                        }
                    },
                    publisher: {
                        match value.0.publisher.len() {
                            0 => None,
                            _ => Some(join_string_vec(value.0.publisher))
                        }
                    },
                    school: {
                        match value.0.school.len() {
                            0 => None,
                            _ => Some(join_string_vec(value.0.school))
                        }
                    },
                })
            }
        }
    };
}

try_into_dblp_record! {Article, PublicationRecord::Article}
try_into_dblp_record! {InProceeding, PublicationRecord::InProceeding}
try_into_dblp_record! {Proceeding, PublicationRecord::Proceeding}
try_into_dblp_record! {Book, PublicationRecord::Book}
try_into_dblp_record! {InCollection, PublicationRecord::InCollection}
try_into_dblp_record! {PhdThesis, PublicationRecord::PhdThesis}
try_into_dblp_record! {MastersThesis, PublicationRecord::MastersThesis}
try_into_dblp_record! {Data, PublicationRecord::Data}

// impl TryFrom<Article> for DblpRecord {
//     type Error = ();
//     fn try_from(value: Article) -> Result<Self, Self::Error> {
//         Ok(Self {
//             record: todo!(),
//             key: value.0.key,
//             mdate: value.0.mdate.and_then(|d| Some(d.to_string())),
//             publtype: value.0.publtype,
//             year: value.0.year,
//             authors: value
//                 .0
//                 .authors
//                 .iter()
//                 .map(|a| a.name.clone())
//                 .collect::<Vec<_>>(),
//             citations: value.0.citations,
//             publisher: value.0.publisher,
//             school: value.0.school,
//         })
//     }
// }

impl TryFrom<WebPage> for PersonRecord {
    type Error = ();

    fn try_from(value: WebPage) -> Result<Self, Self::Error> {
        match value.title.first().and_then(|t| Some(t.as_str())) {
            Some("Home Page") => (),
            _ => return Err(()),
        }

        let author = value.author.first().cloned().unwrap_or_default();
        let aliases = value
            .author
            .iter()
            .skip(1)
            .map(|a| a.to_string())
            .collect::<Vec<_>>();

        Ok(Self {
            // it is safe to set this to 0 as instances
            // of this struct will not use this field.
            id: 0,
            name: author,
            profile: value.key,
            aliases: join_string_vec(aliases),
        })
    }
}

macro_rules! extend_publications {
    ($source: expr, $target: ident) => {
        let iterator = $source
            .into_iter()
            .filter_map(|a| DblpRecord::try_from(a).ok());
        $target.extend(iterator);
    };
}

impl From<RawDblp> for (Vec<DblpRecord>, Vec<PersonRecord>) {
    fn from(value: RawDblp) -> Self {
        let mut publications = Vec::new();
        let mut persons = Vec::new();

        extend_publications!(value.articles, publications);
        extend_publications!(value.inproceedings, publications);
        extend_publications!(value.proceedings, publications);
        extend_publications!(value.books, publications);
        extend_publications!(value.incollections, publications);
        extend_publications!(value.phd_theses, publications);
        extend_publications!(value.masters_theses, publications);

        let ppl = value
            .web_pages
            .into_iter()
            .filter_map(|web| PersonRecord::try_from(web).ok());
        persons.extend(ppl);

        (publications, persons)
    }
}

impl Display for PublicationRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicationRecord::Article => write!(f, "article"),
            PublicationRecord::InProceeding => write!(f, "inproceeding"),
            PublicationRecord::Proceeding => write!(f, "proceeding"),
            PublicationRecord::InCollection => write!(f, "incollection"),
            PublicationRecord::Book => write!(f, "book"),
            PublicationRecord::Collection => write!(f, "collection"),
            PublicationRecord::PhdThesis => write!(f, "phdthesis"),
            PublicationRecord::MastersThesis => write!(f, "mastersthesis"),
            PublicationRecord::Data => write!(f, "data"),
        }
    }
}

impl FromStr for PublicationRecord {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "article" => Ok(PublicationRecord::Article),
            "inproceeding" => Ok(PublicationRecord::InProceeding),
            "proceeding" => Ok(PublicationRecord::Proceeding),
            "incollection" => Ok(PublicationRecord::InCollection),
            "book" => Ok(PublicationRecord::Book),
            "collection" => Ok(PublicationRecord::Collection),
            "phdthesis" => Ok(PublicationRecord::PhdThesis),
            "mastersthesis" => Ok(PublicationRecord::MastersThesis),
            "data" => Ok(PublicationRecord::Data),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::dataset::strip_references;

    use super::*;

    #[test]
    fn test_conversion_from_xml() {
        let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();

        let filtered = strip_references(&contents);

        let dblp: RawDblp = quick_xml::de::from_str(&filtered).unwrap();

        let (publications, persons): (Vec<DblpRecord>, Vec<PersonRecord>) = dblp.into();

        println!("num articles: {}", publications.len());
        println!("num persons: {}", persons.len());
    }

    #[test]
    fn test_serialize_temporal_relation() {
        let x = PersonTemporalRelation {
            author: "jeff".to_string(),
            years: (2000, 2001),
            coauthor_years: vec![
                HashSet::from(["coauthor".to_string(), "coauthor2".to_string()]),
                HashSet::from(["coauthor".to_string(), "new_coauthor".to_string()]),
            ],
        };

        println!("{:?}", x.to_csv_headers());
        println!("{:?}", x.to_csv_row());
    }
}
