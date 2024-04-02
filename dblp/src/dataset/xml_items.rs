//! Concrete definitions for parsing the DBLP dataset.
//!
//! XML schema description found here: https://dblp.org/faq/16154937.html

use std::{collections::HashMap, str::SplitTerminator};

use serde::{Deserialize, Serialize};

/// Raw data derserialized from the DBLP `xml` dataset.
#[derive(Debug, Serialize, Deserialize)]
pub struct RawDblp {
    #[serde(rename = "article")]
    #[serde(default)]
    pub articles: Vec<Article>,

    #[serde(default)]
    pub inproceedings: Vec<InProceeding>,

    #[serde(default)]
    pub proceedings: Vec<Proceeding>,

    #[serde(rename = "book")]
    #[serde(default)]
    pub books: Vec<Book>,

    #[serde(rename = "incollection")]
    #[serde(default)]
    pub incollections: Vec<InCollection>,

    #[serde(rename = "phdthesis")]
    #[serde(default)]
    pub phd_theses: Vec<PhdThesis>,

    #[serde(rename = "mastersthesis")]
    #[serde(default)]
    pub masters_theses: Vec<MastersThesis>,

    /// wtf is this??? Not mentioned in DBLP at all.
    #[serde(default)]
    pub data: Vec<Data>,

    /// Web pages. Also happens to contain person records due to "BACKWARDS COMPATIBILITY"
    #[serde(rename = "www")]
    #[serde(default)]
    pub web_pages: Vec<WebPage>,

    /// Date last modified, for the entire dataset.
    pub mdate: Option<chrono::NaiveDate>,
}

/// Common internal representation of a publication record.
#[derive(Debug, Serialize, Deserialize)]
struct PublicationRecord {
    /// Unique key for the record
    #[serde(rename = "@key")]
    key: String,

    /// Date last modified
    #[serde(rename = "@mdate")]
    mdate: Option<chrono::NaiveDate>,

    /// Space separated tags specifying the type of record
    #[serde(rename = "@publtype")]
    publtype: Option<String>,

    // start of elements
    /// Year of publication
    pub year: Option<u32>,

    /// List of authors
    #[serde(rename = "author")]
    #[serde(default)]
    pub authors: Vec<Author>,

    /// Relation to other records
    #[serde(rename = "rel")]
    pub relation: Option<Relation>,

    /// School where the work was done
    pub school: Option<String>,

    pub publisher: Option<String>,

    /// Citations in record
    #[serde(rename = "cite")]
    #[serde(default)]
    pub citations: Vec<String>,
}

/// These are the `www` items in the DBLP dataset.
/// We only require the path to the author's profile and the author's name.
///
/// Deser info taken from: https://dblp.org/faq/1474690.html
#[derive(Debug, Serialize, Deserialize)]
pub struct WebPage {
    /// The path to the author's profile. unique to each author.
    #[serde(rename = "@key")]
    key: String,

    /// For person records, this is always "Home Page".
    title: Option<String>,

    /// Url of the page
    url: Option<String>,

    /// Author may have multiple aliases, cause idk.
    /// The first element is the current name.
    #[serde(default)]
    author: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
    #[serde(rename = "$value")]
    pub key: String,
    pub mdate: Option<chrono::NaiveDate>,
    pub cdate: Option<String>,
    pub authors: Vec<Author>,
    pub crossref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    #[serde(rename = "$value")]
    pub name: String,
    #[serde(rename = "@aux")]
    pub aux: Option<String>,
    #[serde(rename = "@bibtex")]
    pub bibtex: Option<String>,
    #[serde(rename = "@orcid")]
    pub orcid: Option<String>,
    #[serde(rename = "@label")]
    pub label: Option<String>,
}

/// Any related items to the publication record.
#[derive(Debug, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "@type")]
    pub ty: Option<String>,
    #[serde(rename = "@uri")]
    pub uri: Option<String>,
    #[serde(rename = "@label")]
    pub label: Option<String>,
    #[serde(rename = "@sort")]
    pub sort: Option<u32>,
}

/// A citation
#[derive(Debug, Serialize, Deserialize)]
pub struct Citation {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Article(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct InProceeding(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct Proceeding(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct Book(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct InCollection(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct PhdThesis(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct MastersThesis(PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct Data(PublicationRecord);

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_deserialize_sample() {
        let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();

        let filtered = super::super::Dblp::strip_references(&contents);

        let dblp: RawDblp = quick_xml::de::from_str(&filtered).unwrap();
        // println!("{:#?}", dblp);

        println!("num articles: {}", dblp.articles.len());
        println!("num inproceedings: {}", dblp.inproceedings.len());
        println!("num proceedings: {}", dblp.proceedings.len());
        println!("num books: {}", dblp.books.len());
        println!("num incollections: {}", dblp.incollections.len());
        println!("num phd theses: {}", dblp.phd_theses.len());
        println!("num masters theses: {}", dblp.masters_theses.len());
        println!("num web pages: {}", dblp.web_pages.len());

        println!("{:#?}", dblp.inproceedings);
        println!("{:#?}", dblp.articles);

        let x = dblp
            .incollections
            .iter()
            .filter(|incol| incol.0.citations.len() != 0)
            .collect::<Vec<_>>();

        println!("{:#?}", x);
    }
}
