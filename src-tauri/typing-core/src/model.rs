//! Shared practice, collection, and error types.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

/// Order used to present the items inside one chapter.
///
/// Random order does not move the saved collection cursor. That cursor is the
/// word-book progress position, so changing this setting cannot relocate a
/// collection in the library.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PracticeOrder {
    /// Present the chapter in the collection's original order.
    #[default]
    Book,
    /// Present the same chapter items in a stable shuffled order.
    Random,
}

/// Text kept for the current practice item, plus the attempt counters.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Practice {
    /// Characters still present after edits.
    pub text: String,
    /// Characters submitted, including ones later removed.
    pub attempts: u32,
    /// Submitted characters that matched the target at that position.
    pub correct_attempts: u32,
}

/// Progress of one practice item at a point in time.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    /// Matching characters in the current text.
    pub correct: u32,
    /// Whole-number percentage of the target completed.
    pub percent: u32,
    /// Whole-number percentage of submitted characters that were correct.
    pub accuracy: u32,
    /// Correct characters per minute. Zero until one second has elapsed.
    pub cpm: u32,
    /// Whether the current text is exactly the target.
    pub done: bool,
}

/// Practice mode selected by the user.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PracticeMode {
    /// Practice one sentence from an article at a time.
    #[default]
    Sentence,
    /// Practice one English word from a word book at a time.
    Word,
}

/// A sentence article owned by the Rust content model.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    /// Stable identifier for this article.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Source or origin note shown in settings.
    pub source: String,
    /// Sentence units in their original order.
    pub sentences: Vec<String>,
    /// Chinese glosses aligned with `sentences`. Empty when the article has none.
    #[serde(default)]
    pub translations: Vec<String>,
    /// Chapter titles. Empty means one untitled chapter containing every sentence.
    #[serde(default)]
    pub chapter_titles: Vec<String>,
    /// Exclusive sentence indexes where each chapter ends.
    #[serde(default)]
    pub chapter_ends: Vec<usize>,
}

/// One flat word record used by the practice UI.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordEntry {
    /// English word to type and pronounce.
    pub name: String,
    /// Translations for the word.
    pub trans: Vec<String>,
    /// American pronunciation.
    pub usphone: String,
    /// British pronunciation.
    pub ukphone: String,
}

/// A word book owned by the Rust content model.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordBook {
    /// Stable identifier for this word book.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Source or origin note shown in settings.
    pub source: String,
    /// Word records in their original order.
    pub words: Vec<WordEntry>,
}

/// A compact content entry used for selectors.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummary {
    /// Stable collection identifier.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Number of practice items.
    pub count: usize,
    /// Whether this is the currently selected collection.
    pub selected: bool,
    /// Source note for the collection.
    pub source: String,
    /// Number of chapters in the collection.
    pub chapter_count: usize,
    /// Percentage of items already passed in this collection, from 0 to 100.
    pub progress: u32,
    /// Whether the user can remove this collection. Built-in collections cannot be removed.
    pub removable: bool,
}

/// Content selectors and available collection summaries.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentCatalogSnapshot {
    /// Current practice mode.
    pub mode: PracticeMode,
    /// Available sentence articles.
    pub articles: Vec<CollectionSummary>,
    /// Available word books.
    pub word_books: Vec<CollectionSummary>,
}

/// A complete snapshot rendered by the desktop or web UI.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSnapshot {
    /// Current practice mode.
    pub mode: PracticeMode,
    /// Current collection identifier.
    pub collection_id: String,
    /// Current collection title.
    pub collection_title: String,
    /// One-based current item position.
    pub item_index: usize,
    /// Number of items in the current collection.
    pub item_count: usize,
    /// One-based current chapter number.
    pub chapter_index: usize,
    /// Number of chapters in the current collection.
    pub chapter_count: usize,
    /// One-based word or item position inside the current chapter.
    pub chapter_item_index: usize,
    /// Number of items in the current chapter.
    pub chapter_item_count: usize,
    /// Completed progress for the current chapter.
    pub chapter_progress: u32,
    /// Whether the current item is the final item in its chapter.
    pub is_chapter_last_item: bool,
    /// Whether the current item is the final item in the collection.
    pub is_last_item: bool,
    /// Text that must be typed for the current item.
    pub target: String,
    /// Word metadata when the current mode is word.
    pub word: Option<WordEntry>,
    /// Title of the current chapter. Empty when chapters are only numbered.
    pub chapter_title: String,
    /// Every chapter title. Empty when the collection uses numbered chapters.
    pub chapter_titles: Vec<String>,
    /// Gloss shown under the current sentence. Empty for words and plain articles.
    pub translation: String,
    /// Current typing counters.
    pub practice: Practice,
    /// Current typing statistics.
    pub stats: Statistics,
}

/// Serializable user content and selection state stored in the app data directory.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredContentState {
    /// User imported articles.
    #[serde(default)]
    pub articles: Vec<Article>,
    /// User imported word books.
    #[serde(default)]
    pub word_books: Vec<WordBook>,
    /// Last selected practice mode.
    #[serde(default)]
    pub mode: PracticeMode,
    /// Last selected article identifier.
    #[serde(default)]
    pub article_id: Option<String>,
    /// Last selected word book identifier.
    #[serde(default)]
    pub word_book_id: Option<String>,
    /// Zero-based sentence position for the selected article.
    #[serde(default)]
    pub sentence_index: usize,
    /// Zero-based word position for the selected word book.
    #[serde(default)]
    pub word_index: usize,
    /// Zero-based resume position for every collection, keyed by collection id.
    #[serde(default)]
    pub positions: BTreeMap<String, usize>,
}

/// A successful article or word book import preview.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Display title derived from the file name.
    pub title: String,
    /// Number of usable items.
    pub count: usize,
    /// Number of duplicate word rows ignored after the first occurrence.
    pub duplicate_count: usize,
    /// First sentence or word, when available.
    pub first_item: Option<String>,
}

/// Errors produced while cleaning, parsing, selecting, or reading content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentError {
    /// The supplied article has no usable text.
    EmptyArticle,
    /// The article has text but no sentence units after cleaning.
    NoSentences,
    /// The word table has no header row.
    MissingHeader,
    /// The word table header is invalid.
    InvalidHeader(String),
    /// A word table row is invalid.
    InvalidWordRow {
        /// One-based input line number.
        line: usize,
        /// Human-readable validation reason.
        reason: String,
    },
    /// A selected collection does not exist or has no items.
    UnknownCollection(String),
    /// The current item is not complete yet.
    ItemNotComplete,
    /// The requested practice item does not exist.
    InvalidItem {
        /// Zero-based item index received from the UI.
        item: usize,
        /// Number of available items.
        count: usize,
    },
    /// The requested word chapter does not exist.
    InvalidChapter {
        /// Zero-based chapter index received from the UI.
        chapter: usize,
        /// Number of available chapters.
        count: usize,
    },
    /// The final item in a collection has already been completed.
    EndOfCollection,
    /// The embedded word book cannot be decoded.
    BuiltinWordBook(String),
    /// The embedded sentence collection cannot be decoded.
    BuiltinArticle(String),
    /// Persistent content could not be read or written.
    Storage(String),
    /// A built-in collection cannot be removed.
    NotRemovable,
}

impl Display for ContentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyArticle => formatter.write_str("文章正文为空"),
            Self::NoSentences => formatter.write_str("清理后没有可练习的句子"),
            Self::MissingHeader => formatter.write_str("词表缺少表头"),
            Self::InvalidHeader(reason) => write!(formatter, "词表表头无效：{reason}"),
            Self::InvalidWordRow { line, reason } => {
                write!(formatter, "第 {line} 行无效：{reason}")
            }
            Self::UnknownCollection(id) => write!(formatter, "找不到内容：{id}"),
            Self::ItemNotComplete => formatter.write_str("当前练习项尚未完成"),
            Self::InvalidItem { item, count } => {
                write!(formatter, "找不到第 {} 项，共 {count} 项", item + 1)
            }
            Self::InvalidChapter { chapter, count } => {
                write!(formatter, "找不到第 {} 章，共 {count} 章", chapter + 1)
            }
            Self::EndOfCollection => formatter.write_str("本词书已完成"),
            Self::BuiltinWordBook(reason) => write!(formatter, "内置词书无法读取：{reason}"),
            Self::BuiltinArticle(reason) => write!(formatter, "内置文章无法读取：{reason}"),
            Self::Storage(reason) => write!(formatter, "内容保存失败：{reason}"),
            Self::NotRemovable => formatter.write_str("预置词本不能移除"),
        }
    }
}

impl Error for ContentError {}

/// One chapter window, either authored or numbered.
pub(crate) struct ArticleSpan {
    /// Zero-based chapter index.
    pub(crate) index: usize,
    /// Number of chapters in the collection.
    pub(crate) count: usize,
    /// First collection index included in this chapter.
    pub(crate) start: usize,
    /// Number of items in this chapter.
    pub(crate) len: usize,
    /// Authored chapter title. Empty for a numbered chapter.
    pub(crate) title: String,
}

/// Item currently presented to the typist.
///
/// This is a view of one collection slot. It is not the saved progress cursor.
pub(crate) struct PracticeItem {
    /// Title of the collection that owns the item.
    pub(crate) collection_title: String,
    /// Number of items in that collection.
    pub(crate) item_count: usize,
    /// Text the typist must enter.
    pub(crate) target: String,
    /// Dictionary entry when the item is a word.
    pub(crate) word: Option<WordEntry>,
    /// Gloss shown with a sentence.
    pub(crate) translation: String,
    /// Title of the chapter containing this item.
    pub(crate) chapter_title: String,
    /// Authored chapter titles. Empty when chapters are numbered.
    pub(crate) chapter_titles: Vec<String>,
    /// Zero-based index of the chapter containing this item.
    pub(crate) chapter_index: usize,
    /// Number of chapters in the collection.
    pub(crate) chapter_count: usize,
    /// First collection index included in this chapter.
    pub(crate) chapter_start: usize,
    /// Number of items in this chapter.
    pub(crate) chapter_len: usize,
}
