//! Definitions for use with the embedded sqlite database.
//!
//! Basically, the data types defined in this module are filtered versions of
//! the ones in [super::data_items].

use std::{fmt::Display, fs::Permissions, str::FromStr};

use rusqlite::{types::FromSql, Connection};
use serde::{Deserialize, Serialize};

use super::xml_items::*;

/// The separator used to join vectors in the database.
const SEPARATOR: &str = "::";

/// A single entry in the database.
/// Each publication type is squashed into the "record" field.
///
/// Vectors are joined with double colons: "::"
#[derive(Debug, Serialize, Deserialize)]
pub struct DblpRecord {
    pub record: PublicationRecord,

    /// The key of the publication record.
    /// Can be a conference, journal, etc.
    pub key: String,
    pub mdate: Option<String>,
    pub publtype: Option<String>,

    pub year: Option<u32>,

    /// Authors are referenced by their profile.
    pub authors: Option<String>,

    /// Other publications referenced by this publication.
    /// Publications are referenced by their key.
    pub citations: Option<String>,

    /// Publisher of the publication.
    pub publisher: Option<String>,

    pub school: Option<String>,
}

/// The type of publication record in the database.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct PersonRecord {
    pub name: String,

    /// This is unique across all person records.
    /// Used to distinguish between people.
    pub profile: String,

    /// Other names the person is known by.
    pub aliases: String,
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
                        .collect::<Vec<_>>()
                        .join(SEPARATOR);

                        match val.len() {
                            0 => None,
                            _ => Some(val)
                        }
                    },
                    citations: {
                        let val = value.0.citations.join(SEPARATOR);
                        match val.len() {
                            0 => None,
                            _ => Some(val)
                        }
                    },
                    publisher: {
                        let val = value.0.publisher.join(SEPARATOR);
                        match val.len() {
                            0 => None,
                            _ => Some(val)
                        }
                    },
                    school: {
                        let val = value.0.school.join(SEPARATOR);
                        match val.len() {
                            0 => None,
                            _ => Some(val)
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
            .collect::<Vec<_>>()
            .join(SEPARATOR);

        Ok(Self {
            name: author,
            profile: value.key,
            aliases,
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
