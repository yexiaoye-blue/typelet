//! Content ownership and typing state for Typelet.
//!
//! The desktop application sends user input and operations to this crate. The
//! crate selects the current item, parses imported content, updates attempts,
//! and returns serializable snapshots for the UI.

#![deny(missing_docs)]

mod builtin;
mod catalog;
mod chapter;
mod model;
mod parse;
mod scoring;
mod session;

pub use catalog::ContentCatalog;
pub use model::{
    Article, CollectionSummary, ContentCatalogSnapshot, ContentError, ImportReport, Practice,
    PracticeMode, PracticeOrder, PracticeSnapshot, Statistics, StoredContentState, WordBook,
    WordEntry,
};
pub use parse::{parse_article, parse_word_book};
pub use scoring::{statistics, update_practice};
pub use session::PracticeSession;

#[cfg(test)]
mod tests;
