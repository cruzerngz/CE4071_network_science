//! Definitions for use with the embedded sqlite database

use serde::{Deserialize, Serialize};

use super::data_items::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct DblpRecord {
    pub record: PublicationRecord,
    pub key: String,
    pub mdate: Option<String>,
    pub publtype: Option<String>,
}

/// The type of publication record in the database.
#[derive(Debug, Serialize, Deserialize)]
pub enum PublicationRecord {
    Article,
    InProceeding,
    Proceeding,
    Book,
    Collection,
    PhdThesis,
    MastersThesis,
}
