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
struct ChunkedXmlViewer<'xml> {
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

impl<'xml> ChunkedXmlViewer<'xml> {
    const XML_START_TAG: &'static str = r"<\w+>|<\w+";
    const XML_END_TAG: &'static str = r"</\w+>|/>";
    const XML_SELF_CLOSE_TAG: &'static str = r"<\w+.?/>";

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

        let start_regex = Regex::new(Self::XML_START_TAG).expect("regex compilation must not fail");
        let end_regex = Regex::new(Self::XML_END_TAG).expect("regex compilation must not fail");

        let start_pos = start_regex
            .find(&self.inner[self.offset..])
            .expect("no start tag found");
        let end_pos = end_regex
            .find(&self.inner[self.offset..])
            .expect("no end tag found");

        let start = start_pos.start();
        let end = end_pos.end();

        let chunk = &self.inner[start..end];

        self.offset += end;

        todo!()
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

            let start = self.re_start.find(reference)?;
            let end = self.re_end.find(reference)?;
            let self_close = self.re_self_close.find(reference);

            match self_close {
                Some(s_close) => match (
                    s_close.start().cmp(&start.start()),
                    s_close.start().cmp(&end.start()),
                ) {
                    // self closing tags only affect the offset
                    (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => {
                        // println!("self closing tag: {}", s_close.as_str());
                        offset += s_close.end();
                        reference = &reference[s_close.end()..];
                        continue;
                    }
                    _ => (),
                },
                None => (),
            }

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
    use super::*;

    #[test]
    fn test_init_chunk_viewer() {
        let xml_file = fs::read_to_string("dblp_trunc.xml").unwrap();

        let mut viewer = ChunkedXmlViewer::from_str(&xml_file[..2000], 10);

        while let Some(chunk) = viewer.next_element() {
            println!("chunk: {}", chunk);
        }
    }
}
