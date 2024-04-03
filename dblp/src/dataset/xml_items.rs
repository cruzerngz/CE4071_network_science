//! Concrete definitions for parsing the DBLP dataset.
//!
//! XML schema description found here: https://dblp.org/faq/16154937.html

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
pub struct PublicationRecord {
    /// Unique key for the record
    #[serde(rename = "@key")]
    pub key: String,

    /// Date last modified
    #[serde(rename = "@mdate")]
    pub mdate: Option<chrono::NaiveDate>,

    /// Space separated tags specifying the type of record
    #[serde(rename = "@publtype")]
    pub publtype: Option<String>,

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
    #[serde(default)]
    pub school: Vec<String>,

    #[serde(default)]
    pub publisher: Vec<String>,

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
    pub key: String,

    /// For person records, this is always "Home Page".
    #[serde(default)]
    pub title: Vec<String>,

    /// Url of the page
    #[serde(default)]
    pub url: Vec<String>,

    /// Author may have multiple aliases, cause idk.
    /// The first element is the current name.
    #[serde(default)]
    pub author: Vec<String>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Article(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct InProceeding(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct Proceeding(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct Book(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct InCollection(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct PhdThesis(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct MastersThesis(pub(crate) PublicationRecord);

#[derive(Debug, Serialize, Deserialize)]
pub struct Data(pub(crate) PublicationRecord);

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_deserialize_sample() {
        let contents = std::fs::read_to_string("dblp_trunc.xml").unwrap();

        let filtered = super::super::strip_references(&contents);

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

        let x = dblp
            .incollections
            .iter()
            .filter(|incol| incol.0.citations.len() != 0)
            .collect::<Vec<_>>();

        println!("{:#?}", x);
    }

    #[test]
    fn test_deserialize_section() {
        let xml = r#"<dblp><incollection mdate="2017-07-12" key="reference/crypt/Canteaut11" publtype="encyclopedia">
        <author>Anne Canteaut</author>
        <title>A5/1.</title>
        <pages>1-2</pages>
        <year>2011</year>
        <booktitle>Encyclopedia of Cryptography and Security (2nd Ed.)</booktitle>
        <ee>https://doi.org/10.1007/978-1-4419-5906-5_332</ee>
        <crossref>reference/crypt/2011</crossref>
        <url>db/reference/crypt/crypt2011.html#Canteaut11</url>
        <cite>conf/crypto/BarkanBK03</cite>
        <cite>conf/sacrypt/BarkanB05</cite>
        <cite>journals/joc/BarkanBK08</cite>
        <cite>conf/indocrypt/BihamD00</cite>
        <cite>conf/fse/BiryukovSW00</cite>
        <cite>journals/tit/EkdahlJ03</cite>
        <cite>conf/sacrypt/MaximovJB04</cite>
        <cite>...</cite>
        <cite>...</cite>
        </incollection></dblp>"#;

        let dblp: RawDblp = quick_xml::de::from_str(xml).unwrap();

        println!("{:#?}", dblp);
    }
}
