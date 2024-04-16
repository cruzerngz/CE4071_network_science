//! Definitions for use with the embedded sqlite database.
//!
//! Basically, the data types defined in this module are filtered versions of
//! the ones in [super::data_items].

use std::{fmt::Display, str::FromStr};

use pyo3::{pyclass, pymethods, PyRef, PyRefMut};
use serde::{Deserialize, Serialize};

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
}
