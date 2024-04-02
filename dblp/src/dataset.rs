#![allow(unused)]

mod xml_items;
mod db_items;

use std::{
    fs,
    io::{self, Read},
    sync::OnceLock,
    time::Duration,
};

use regex::Regex;

const DBLP_FILE: &str = "dblp.xml.gz";

/// Matcher for XML references
/// They follow this format:
/// &xxxxx;
const XML_REF_REGEX: &str = "&[[:alpha:]]+;";

pub static DBLP_DATABASE: OnceLock<Dblp> = OnceLock::new();

#[derive(Debug)]
pub struct Dblp {
    /// Will expand out to become a ~3GB XML file
    data: Vec<u8>,
}

impl Dblp {
    pub fn new(path: String) -> io::Result<Self> {
        // check if a file exists
        let zipped_contents = match fs::metadata(DBLP_FILE).is_ok() {
            true => {
                let mut file = fs::File::open(DBLP_FILE).unwrap();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).unwrap();

                buffer
            }
            false => return Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
        };

        let mut decoder = flate2::read::GzDecoder::new(zipped_contents.as_slice());

        let mut unzipped = Vec::new();
        decoder.read_to_end(&mut unzipped).unwrap();

        Ok(Self { data: unzipped })
    }

    /// This method ingests the entire XML dataset and strips all
    /// references "$Agrage;" from it. This is performed before deserialization.
    ///
    /// We will use the .dtd file to determine which references to strip.
    ///
    /// Gawddamn pesky
    fn strip_references(input_xml: &str) -> String {
        let regex = Regex::new(XML_REF_REGEX).expect("regex compilation must not fail");

        let res = regex.replace_all(input_xml, "");

        res.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dblp_new() {
        println!("fetching data from DBLP...");
        let dblp = Dblp::new(DBLP_FILE.to_string()).expect("file should exist");
        println!("done!");
        println!("num bytes: {:?}", dblp.data.len());
        assert!(dblp.data.len() > 0);
    }

    #[test]
    fn test_strip_dblp_refs() {
        let contents = fs::read_to_string("dblp_head.xml").unwrap();

        let stripped = Dblp::strip_references(&contents);

        fs::write("dblp_head_stripped.xml", stripped).unwrap()
    }
}
