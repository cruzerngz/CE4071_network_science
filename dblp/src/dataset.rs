#![allow(unused)]

pub mod db_items;
pub mod xml_items;

use std::{
    fs,
    io::{self, Read},
    sync::OnceLock,
    time::Duration,
};

use chrono::naive::serde::ts_seconds_option;
use regex::Regex;

const DBLP_FILE: &str = "dblp.xml.gz";

/// Matcher for XML references
/// They follow this format:
/// &xxxxx;
const XML_REF_REGEX: &str = "&[[:alpha:]]+;";

/// This method ingests the entire XML dataset and strips all
/// references "$Agrage;" from it. This is performed before deserialization.
///
/// We will use the .dtd file to determine which references to strip.
///
/// Gawddamn pesky
pub fn strip_references(input_xml: &str) -> String {
    let regex = Regex::new(XML_REF_REGEX).expect("regex compilation must not fail");

    let res = regex.replace_all(input_xml, "");

    res.into_owned()
}

/// An XML viewer that reads the XML in chunks.
/// XML that is parsed must be valid.
///
/// Each chunk is guaranteed to be valid XML.
///
/// XML tags reference: https://www.w3.org/TR/REC-xml/#sec-starttags
#[derive(Debug)]
pub struct ChunkedXmlViewer<'xml> {
    offset: usize,
    len: usize,
    num_chunks: usize,

    // copies of the root tag are needed
    root_tag_start: String,
    root_tag_end: String,

    re_start: Regex,
    re_end: Regex,
    re_self_close: Regex,

    inner: &'xml str,
}

impl Iterator for ChunkedXmlViewer<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_chunk()
    }
}

impl<'xml> ChunkedXmlViewer<'xml> {
    const XML_START_TAG: &'static str = r"<\w+>|<\w+";
    const XML_END_TAG: &'static str = r"</\w+>|/>";
    const XML_SELF_CLOSE_TAG: &'static str = r"<.*?/>";

    /// `num_chunks` specifies the number of level 1 XML elements to read in a single iteration.
    pub fn from_str(input: &'xml str, num_chunks: usize) -> Self {
        // let start_tag;

        let start_regex = Regex::new(Self::XML_START_TAG).expect("regex compilation must not fail");
        let end_regex = Regex::new(Self::XML_END_TAG).expect("regex compilation must not fail");
        let self_close_regex =
            Regex::new(Self::XML_SELF_CLOSE_TAG).expect("regex compilation must not fail");

        let pos = start_regex.find(input).expect("no start tag found");

        let tag_start = pos.as_str().to_owned();
        let tag_end = format!("</{}>", pos.as_str().trim_matches(['<', '>']).to_string());

        Self {
            offset: 0,
            len: input.len() - pos.end(),
            num_chunks,
            root_tag_start: tag_start,
            root_tag_end: tag_end,
            re_start: start_regex,
            re_end: end_regex,
            re_self_close: self_close_regex,

            // straight to the first level 1 element
            inner: &input[pos.end()..],
        }
    }

    pub fn next_chunk(&mut self) -> Option<String> {
        if self.offset >= self.len {
            return None;
        }

        let mut count = self.num_chunks;
        let mut chunks = Vec::new();

        while count != 0 {
            match self.next_element() {
                Some(chunk) => {
                    chunks.push(chunk);
                    count -= 1;
                }
                None => break,
            }
        }

        match chunks.len() {
            0 => None,
            _ => Some(format!(
                "{}{}{}",
                self.root_tag_start,
                chunks.join(""),
                self.root_tag_end
            )),
        }
    }

    /// Returns the next element in the XML, without performing allocation.
    pub fn next_element(&mut self) -> Option<&'xml str> {
        let mut depth = 0;
        let mut offset = 0;

        // starting point
        let reference = &self.inner[self.offset..];

        // start by pushing the first starting tag
        let start = self.re_start.find(reference)?;

        depth += 1;
        offset += start.end();

        let mut reference = &reference[start.end()..];

        while depth != 0 {
            // println!("element depth: {}", depth);
            println!("matching regexes...");
            println!("remaining length: {}, peek: {}",reference.len(), &reference[..10]);

            let start = self.re_start.find(reference).unwrap();
            let end = self.re_end.find(reference).unwrap();
            let self_close = self.re_self_close.find(reference);

            // handle self closing tags
            match self_close {
                Some(s_close) => match (
                    s_close.start().cmp(&start.start()),
                    s_close.start().cmp(&end.start()),
                ) {
                    // self closing tags only affect the offset
                    (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => {
                        println!("self closing tag: {}", s_close.as_str());

                        // println!("self closing tag: {}", s_close.as_str());
                        offset += s_close.end();
                        reference = &reference[s_close.end()..];
                        continue;
                    }
                    _ => (),
                },
                None => (),
            }

            // handle start and end tags
            println!("handling start and end tags...");
            match start.start().cmp(&end.start()) {
                // start tag found
                std::cmp::Ordering::Less => {
                    // println!("opening tag: {}", start.as_str());
                    depth += 1;
                    offset += start.end();
                    reference = &reference[start.end()..];
                }
                // end tag found
                std::cmp::Ordering::Greater => {
                    // println!("closing tag: {}", end.as_str());
                    depth -= 1;
                    offset += end.end();
                    reference = &reference[end.end()..];
                }
                std::cmp::Ordering::Equal => {
                    unimplemented!("both regex cannot match at the same position")
                }
            }
        }

        let res = Some(&self.inner[self.offset..(self.offset + offset)]);
        self.offset += offset;

        res
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{chunked_deserialize_insert, clear_tables, create_tables};

    use self::xml_items::RawDblp;

    use super::*;

    #[test]
    fn test_match_regex() {
        let re_start = Regex::new(ChunkedXmlViewer::XML_START_TAG).unwrap();
        let re_end = Regex::new(ChunkedXmlViewer::XML_END_TAG).unwrap();
        let re_self_close = Regex::new(ChunkedXmlViewer::XML_SELF_CLOSE_TAG).unwrap();

        let start_tags = &["<open asd='123'>", "<open>"];
        let end_tags = &["</close>", "/>"];
        let self_close_tags = &["<self_close/>", "<self_close asd='123'/>"];

        for tag in start_tags {
            assert!(re_start.is_match(tag));
        }

        for tag in end_tags {
            assert!(re_end.is_match(tag));
        }

        for tag in self_close_tags {
            assert!(re_self_close.is_match(tag));
        }
    }

    #[test]
    fn test_chunk_viewer() {
        let xml_file = fs::read_to_string("dblp.xml").unwrap();

        let mut viewer = ChunkedXmlViewer::from_str(&xml_file, 10);

        while let Some(elem) = viewer.next_element() {
            println!("{}", elem);
        }

        // for chunk in viewer {
        //     let raw_data: RawDblp = quick_xml::de::from_str(&chunk).unwrap();
        // }
    }

    #[test]
    fn test_chunked_write_to_db() {
        let xml_file = fs::read_to_string("dblp.xml").unwrap();
        let filtered = strip_references(&xml_file);

        let mut conn = rusqlite::Connection::open("temp.sqlite").unwrap();
        create_tables(&conn).unwrap();
        clear_tables(&conn).unwrap();
        create_tables(&conn).unwrap();

        chunked_deserialize_insert(&mut conn, &filtered).unwrap();
    }
}
