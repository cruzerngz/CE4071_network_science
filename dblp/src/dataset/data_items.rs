//! Concrete definitions of the data items that are used in `dblp`.
//!

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Dblp {
    // #[serde(rename = "$value")]
    // pub articles: Vec<Article>,
    // #[serde(rename = "$value")]
    // pub inproceedings: Vec<InProceedings>,
    // pub proceedings: Vec<Proceedings>,
    // pub books: Vec<Book>,
    #[serde(rename = "$value")]
    #[serde(default)]
    pub incollections: Vec<InCollection>,
    // pub phdtheses: Vec<PhdThesis>,
    // pub masterstheses: Vec<MastersThesis>,


    #[serde(rename = "$value")]
    pub www: Vec<PersonRecord>,

    // pub persons: Vec<Person>,
    // pub data: Vec<Data>,
    pub mdate: Option<String>,
}

/// These are the `www` items in the DBLP dataset.
/// We only require the path to the author's profile and the author's name.
///
/// Deser info taken from: https://dblp.org/faq/1474690.html
#[derive(Debug, Serialize, Deserialize)]
struct PersonRecord {
    /// The path to the author's profile. unique to each author.
    #[serde(rename = "@key")]
    key: String,

    /// Author may have multiple aliases, cause idk.
    /// The first element is the current name.
    #[serde(default)]
    author: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    #[serde(rename = "$value")]
    pub key: String,
    pub mdate: Option<String>,
    pub cdate: Option<String>,
    pub authors: Vec<Author>,
    pub crossref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Author {
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
    // pub author_type: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Article {
    #[serde(rename = "$value")]
    pub name: String,

    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@mdate")]
    pub mdate: Option<String>,
    #[serde(rename = "@publtype")]
    #[serde(default)]
    pub publtype: Vec<String>,
    #[serde(rename = "@reviewid")]
    pub reviewid: Option<String>,
    #[serde(rename = "@rating")]
    pub rating: Option<String>,
    #[serde(rename = "@cdate")]
    pub cdate: Option<String>,
    // pub fields: Vec<Field>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InProceedings {
    key: String,
    mdate: Option<String>,
    publtype: Option<String>,
    reviewid: Option<String>,
    rating: Option<String>,
    cdate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InCollection {
    #[serde(rename = "@key")]
    key: String,
    #[serde(rename = "@mdate")]
    mdate: Option<String>,
    #[serde(rename = "@publtype")]
    publtype: Option<String>,

    // reviewid: Option<String>,
    // rating: Option<String>,
    // cdate: Option<String>,

    title: String,

    #[serde(default)]
    author: Vec<Author>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Field {
    pub name: String,
    pub content: String,
    pub aux: Option<String>,
}

// Define similar structs for other publication types like InProceedings, Proceedings, Book, etc.

// Define an enum for the various types of fields
#[derive(Debug, Serialize, Deserialize)]
enum FieldType {
    Author,
    Editor,
    Title,
    BookTitle,
    Pages,
    Year,
    Address,
    Journal,
    Volume,
    Number,
    Month,
    Url,
    Ee,
    Cdrom,
    Cite,
    Publisher,
    Note,
    Crossref,
    Isbn,
    Series,
    School,
    Chapter,
    Publnr,
    Stream,
    Rel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_sample() {
        let contents = std::fs::read_to_string("dblp_head.xml").unwrap();

        let dblp: Dblp = quick_xml::de::from_str(&contents).unwrap();


        println!("{:#?}", dblp);
    }
}
