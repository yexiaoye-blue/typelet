//! Content ownership and typing state for Typelet.
//!
//! The desktop application sends user input and operations to this crate. The
//! crate selects the current item, parses imported content, updates attempts,
//! and returns serializable snapshots for the UI.

#![deny(missing_docs)]

use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

const CHAPTER_LENGTH: usize = 20;

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

/// Count only the edited span between previous text and value.
pub fn update_practice(previous: &Practice, value: &str, target: &str) -> Practice {
    let before: Vec<char> = previous.text.nfc().collect();
    let after: Vec<char> = value.nfc().collect();
    let expected: Vec<char> = target.nfc().collect();
    let mut start = 0;
    let mut end_before = before.len();
    let mut end_after = after.len();

    while start < end_before && start < end_after && before[start] == after[start] {
        start += 1;
    }
    while end_before > start && end_after > start && before[end_before - 1] == after[end_after - 1]
    {
        end_before -= 1;
        end_after -= 1;
    }

    let correct = after[start..end_after]
        .iter()
        .enumerate()
        .filter(|(offset, character)| expected.get(start + *offset) == Some(character))
        .count() as u32;

    Practice {
        text: after.into_iter().collect(),
        attempts: previous.attempts + (end_after - start) as u32,
        correct_attempts: previous.correct_attempts + correct,
    }
}

/// Summarize practice against target after milliseconds of practice.
pub fn statistics(practice: &Practice, target: &str, milliseconds: u64) -> Statistics {
    let typed: Vec<char> = practice.text.nfc().collect();
    let expected: Vec<char> = target.nfc().collect();
    let correct = typed
        .iter()
        .enumerate()
        .filter(|(index, character)| expected.get(*index) == Some(character))
        .count() as u32;
    let percent = match u32::try_from(expected.len()) {
        Ok(length) if length > 0 => correct * 100 / length,
        _ => 0,
    };
    let accuracy = if practice.attempts == 0 {
        100
    } else {
        (practice.correct_attempts * 100 + practice.attempts / 2) / practice.attempts
    };
    let cpm = if milliseconds >= 1000 {
        ((correct as u64 * 60_000 + milliseconds / 2) / milliseconds) as u32
    } else {
        0
    };

    Statistics {
        correct,
        percent,
        accuracy,
        cpm,
        done: !expected.is_empty() && typed == expected,
    }
}

/// Clean article text and split it into sentence units.
pub fn parse_article(title: &str, source: &str, text: &str) -> Result<Article, ContentError> {
    let cleaned = clean_article_text(text);
    if cleaned.is_empty() {
        return Err(ContentError::EmptyArticle);
    }
    let sentences = split_sentences(&cleaned);
    if sentences.is_empty() {
        return Err(ContentError::NoSentences);
    }
    let id = stable_id("article", title, &cleaned);
    Ok(Article {
        id,
        title: title.trim().to_owned(),
        source: source.to_owned(),
        sentences,
        translations: Vec::new(),
        chapter_titles: Vec::new(),
        chapter_ends: Vec::new(),
    })
}

/// Parse a UTF-8 TSV word table with a required name column.
pub fn parse_word_book(
    title: &str,
    source: &str,
    text: &str,
) -> Result<(WordBook, ImportReport), ContentError> {
    let content = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = content.lines();
    let Some(header_line) = lines.next() else {
        return Err(ContentError::MissingHeader);
    };
    let headers: Vec<&str> = header_line.split('\t').map(str::trim).collect();
    let mut seen_headers = HashSet::new();
    for header in &headers {
        if !matches!(*header, "name" | "trans" | "usphone" | "ukphone") {
            return Err(ContentError::InvalidHeader((*header).to_owned()));
        }
        if !seen_headers.insert(*header) {
            return Err(ContentError::InvalidHeader(format!("重复字段 {header}")));
        }
    }
    let Some(name_index) = headers.iter().position(|header| *header == "name") else {
        return Err(ContentError::InvalidHeader("缺少 name 字段".to_owned()));
    };

    let mut words = Vec::new();
    let mut names = HashSet::new();
    let mut duplicate_count = 0;
    for (line_offset, raw_line) in lines.enumerate() {
        let line = line_offset + 2;
        if raw_line.trim().is_empty() {
            continue;
        }
        let cells: Vec<&str> = raw_line.split('\t').collect();
        if cells.len() > headers.len()
            && cells[headers.len()..]
                .iter()
                .any(|cell| !cell.trim().is_empty())
        {
            return Err(ContentError::InvalidWordRow {
                line,
                reason: "字段数量多于表头".to_owned(),
            });
        }
        let cell = |field: &str| -> &str {
            headers
                .iter()
                .position(|header| *header == field)
                .and_then(|index| cells.get(index).copied())
                .unwrap_or("")
                .trim()
        };
        let name = cells.get(name_index).copied().unwrap_or("").trim();
        if name.is_empty() {
            return Err(ContentError::InvalidWordRow {
                line,
                reason: "name 不能为空".to_owned(),
            });
        }
        if name.chars().any(char::is_whitespace) {
            return Err(ContentError::InvalidWordRow {
                line,
                reason: "name 不能包含空格".to_owned(),
            });
        }
        if !names.insert(name.to_owned()) {
            duplicate_count += 1;
            continue;
        }
        let trans = cell("trans")
            .split(['；', ';'])
            .map(str::trim)
            .filter(|translation| !translation.is_empty())
            .map(str::to_owned)
            .collect();
        words.push(WordEntry {
            name: name.to_owned(),
            trans,
            usphone: cell("usphone").to_owned(),
            ukphone: cell("ukphone").to_owned(),
        });
    }
    if words.is_empty() {
        return Err(ContentError::InvalidWordRow {
            line: 2,
            reason: "没有可练习的词条".to_owned(),
        });
    }
    let book = WordBook {
        id: stable_id("wordbook", title, text),
        title: title.trim().to_owned(),
        source: source.to_owned(),
        words,
    };
    let report = ImportReport {
        title: book.title.clone(),
        count: book.words.len(),
        duplicate_count,
        first_item: book.words.first().map(|word| word.name.clone()),
    };
    Ok((book, report))
}

/// Store the content collections and the last selected collection.
pub struct ContentCatalog {
    builtin_articles: Vec<Article>,
    builtin_word_book: WordBook,
    articles: Vec<Article>,
    word_books: Vec<WordBook>,
    mode: PracticeMode,
    article_id: String,
    word_book_id: String,
}

impl ContentCatalog {
    /// Create a catalog containing the built-in public-domain excerpts and CET4 book.
    pub fn new() -> Result<Self, ContentError> {
        let builtin_articles = builtin_articles()?;
        let builtin_word_book = builtin_word_book()?;
        let article_id = builtin_articles
            .first()
            .map(|article| article.id.clone())
            .ok_or(ContentError::NoSentences)?;
        Ok(Self {
            builtin_articles,
            article_id,
            builtin_word_book,
            articles: Vec::new(),
            word_books: Vec::new(),
            mode: PracticeMode::Sentence,
            word_book_id: "builtin:cet4_3".to_owned(),
        })
    }

    /// Restore user content and selection from a persisted state.
    pub fn from_stored(stored: StoredContentState) -> Result<Self, ContentError> {
        let mut catalog = Self::new()?;
        catalog.articles = stored.articles;
        catalog.word_books = stored.word_books;
        catalog.mode = stored.mode;
        if let Some(id) = stored.article_id {
            if catalog.article(id.as_str()).is_some() {
                catalog.article_id = id;
            }
        }
        if let Some(id) = stored.word_book_id {
            if catalog.word_book(id.as_str()).is_some() {
                catalog.word_book_id = id;
            }
        }
        Ok(catalog)
    }

    /// Convert this catalog user content and selection to persisted data.
    pub fn stored_state(&self) -> StoredContentState {
        StoredContentState {
            articles: self.articles.clone(),
            word_books: self.word_books.clone(),
            mode: self.mode,
            article_id: Some(self.article_id.clone()),
            word_book_id: Some(self.word_book_id.clone()),
            sentence_index: 0,
            word_index: 0,
        }
    }

    /// Return a compact selector snapshot.
    pub fn snapshot(&self) -> ContentCatalogSnapshot {
        let articles = self
            .builtin_articles
            .iter()
            .map(|article| self.article_summary(article, false))
            .chain(
                self.articles
                    .iter()
                    .map(|article| self.article_summary(article, true)),
            )
            .collect();
        let word_books = std::iter::once(self.word_book_summary(&self.builtin_word_book, false))
            .chain(
                self.word_books
                    .iter()
                    .map(|book| self.word_book_summary(book, true)),
            )
            .collect();
        ContentCatalogSnapshot {
            mode: self.mode,
            articles,
            word_books,
        }
    }

    /// Return the selected mode.
    pub fn mode(&self) -> PracticeMode {
        self.mode
    }

    /// Select a practice mode.
    pub fn set_mode(&mut self, mode: PracticeMode) {
        self.mode = mode;
    }

    /// Select an article by id.
    pub fn select_article(&mut self, id: &str) -> Result<(), ContentError> {
        if self.article(id).is_none() {
            return Err(ContentError::UnknownCollection(id.to_owned()));
        }
        self.article_id = id.to_owned();
        self.mode = PracticeMode::Sentence;
        Ok(())
    }

    /// Select a word book by id.
    pub fn select_word_book(&mut self, id: &str) -> Result<(), ContentError> {
        if self.word_book(id).is_none() {
            return Err(ContentError::UnknownCollection(id.to_owned()));
        }
        self.word_book_id = id.to_owned();
        self.mode = PracticeMode::Word;
        Ok(())
    }

    /// Add or replace an imported article after it has been successfully parsed.
    pub fn add_article(&mut self, article: Article) {
        self.articles.retain(|existing| existing.id != article.id);
        self.article_id = article.id.clone();
        self.articles.push(article);
        self.mode = PracticeMode::Sentence;
    }

    /// Add or replace an imported word book after it has been successfully parsed.
    pub fn add_word_book(&mut self, book: WordBook) {
        self.word_books.retain(|existing| existing.id != book.id);
        self.word_book_id = book.id.clone();
        self.word_books.push(book);
        self.mode = PracticeMode::Word;
    }

    /// Remove an imported article. Built-in articles stay in place.
    pub fn remove_article(&mut self, id: &str) -> Result<(), ContentError> {
        let before = self.articles.len();
        self.articles.retain(|article| article.id != id);
        if self.articles.len() == before {
            return Err(ContentError::NotRemovable);
        }
        if self.article_id == id {
            self.article_id = self
                .builtin_articles
                .first()
                .map(|article| article.id.clone())
                .ok_or(ContentError::NoSentences)?;
        }
        Ok(())
    }

    /// Remove an imported word book. The built-in book stays in place.
    pub fn remove_word_book(&mut self, id: &str) -> Result<(), ContentError> {
        let before = self.word_books.len();
        self.word_books.retain(|book| book.id != id);
        if self.word_books.len() == before {
            return Err(ContentError::NotRemovable);
        }
        if self.word_book_id == id {
            self.word_book_id = self.builtin_word_book.id.clone();
        }
        Ok(())
    }

    fn article_summary(&self, article: &Article, removable: bool) -> CollectionSummary {
        CollectionSummary {
            id: article.id.clone(),
            title: article.title.clone(),
            count: article.sentences.len(),
            selected: self.article_id == article.id,
            source: article.source.clone(),
            chapter_count: article_chapter_count(article),
            removable,
        }
    }

    fn word_book_summary(&self, book: &WordBook, removable: bool) -> CollectionSummary {
        CollectionSummary {
            id: book.id.clone(),
            title: book.title.clone(),
            count: book.words.len(),
            selected: self.word_book_id == book.id,
            source: book.source.clone(),
            chapter_count: chapter_count(book.words.len()),
            removable,
        }
    }

    fn article(&self, id: &str) -> Option<&Article> {
        self.builtin_articles
            .iter()
            .chain(self.articles.iter())
            .find(|article| article.id == id)
    }

    fn word_book(&self, id: &str) -> Option<&WordBook> {
        if self.builtin_word_book.id == id {
            Some(&self.builtin_word_book)
        } else {
            self.word_books.iter().find(|book| book.id == id)
        }
    }

    fn selected_collection_id(&self) -> &str {
        match self.mode {
            PracticeMode::Sentence => &self.article_id,
            PracticeMode::Word => &self.word_book_id,
        }
    }

    fn item_count(&self, mode: PracticeMode, id: &str) -> Result<usize, ContentError> {
        match mode {
            PracticeMode::Sentence => self
                .article(id)
                .map(|article| article.sentences.len())
                .ok_or_else(|| ContentError::UnknownCollection(id.to_owned())),
            PracticeMode::Word => self
                .word_book(id)
                .map(|book| book.words.len())
                .ok_or_else(|| ContentError::UnknownCollection(id.to_owned())),
        }
    }

    fn current_item(
        &self,
        mode: PracticeMode,
        id: &str,
        index: usize,
    ) -> Result<PracticeItem, ContentError> {
        match mode {
            PracticeMode::Sentence => {
                let article = self
                    .article(id)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                let target = article
                    .sentences
                    .get(index)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                let span = article_span(article, index);
                Ok(PracticeItem {
                    collection_title: article.title.clone(),
                    item_count: article.sentences.len(),
                    target: target.clone(),
                    word: None,
                    translation: article.translations.get(index).cloned().unwrap_or_default(),
                    chapter_title: span.title,
                    chapter_titles: article.chapter_titles.clone(),
                    chapter_index: span.index,
                    chapter_count: span.count,
                    chapter_start: span.start,
                    chapter_len: span.len,
                })
            }
            PracticeMode::Word => {
                let book = self
                    .word_book(id)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                let word = book
                    .words
                    .get(index)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                let chapters = chapter_count(book.words.len());
                let current_chapter = index / CHAPTER_LENGTH;
                let start = current_chapter * CHAPTER_LENGTH;
                Ok(PracticeItem {
                    collection_title: book.title.clone(),
                    item_count: book.words.len(),
                    target: word.name.clone(),
                    word: Some(word.clone()),
                    translation: String::new(),
                    chapter_title: String::new(),
                    chapter_titles: Vec::new(),
                    chapter_index: current_chapter,
                    chapter_count: chapters,
                    chapter_start: start,
                    chapter_len: (book.words.len() - start).min(CHAPTER_LENGTH),
                })
            }
        }
    }

    fn chapter_start(
        &self,
        mode: PracticeMode,
        id: &str,
        chapter: usize,
    ) -> Result<usize, ContentError> {
        match mode {
            PracticeMode::Word => {
                let count = self.item_count(mode, id)?;
                let chapters = chapter_count(count);
                if chapter >= chapters {
                    return Err(ContentError::InvalidChapter {
                        chapter,
                        count: chapters,
                    });
                }
                Ok(chapter * CHAPTER_LENGTH)
            }
            PracticeMode::Sentence => {
                let article = self
                    .article(id)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                let chapters = article_chapter_count(article);
                if chapter >= chapters {
                    return Err(ContentError::InvalidChapter {
                        chapter,
                        count: chapters,
                    });
                }
                if article.chapter_ends.is_empty() || chapter == 0 {
                    Ok(0)
                } else {
                    Ok(article.chapter_ends[chapter - 1])
                }
            }
        }
    }
}

struct PracticeItem {
    collection_title: String,
    item_count: usize,
    target: String,
    word: Option<WordEntry>,
    translation: String,
    chapter_title: String,
    chapter_titles: Vec<String>,
    chapter_index: usize,
    chapter_count: usize,
    chapter_start: usize,
    chapter_len: usize,
}

/// Rust-owned current item, position, and typing counters.
#[derive(Clone, Debug)]
pub struct PracticeSession {
    mode: PracticeMode,
    collection_id: String,
    item_index: usize,
    sentence_index: usize,
    word_index: usize,
    practice: Practice,
}

impl PracticeSession {
    /// Start a session at the catalog selected collection and first item.
    pub fn new(catalog: &ContentCatalog) -> Result<Self, ContentError> {
        let collection_id = catalog.selected_collection_id().to_owned();
        catalog.item_count(catalog.mode(), &collection_id)?;
        Ok(Self {
            mode: catalog.mode(),
            collection_id,
            item_index: 0,
            sentence_index: 0,
            word_index: 0,
            practice: Practice::default(),
        })
    }

    /// Resume a saved sentence and word position for the current catalog selection.
    pub fn with_stored_progress(
        catalog: &ContentCatalog,
        sentence_index: usize,
        word_index: usize,
    ) -> Result<Self, ContentError> {
        let mut session = Self::new(catalog)?;
        session.sentence_index = sentence_index;
        session.word_index = word_index;
        session.item_index = match catalog.mode() {
            PracticeMode::Sentence => sentence_index,
            PracticeMode::Word => word_index,
        };
        session.sync_selection(catalog)?;
        Ok(session)
    }

    /// Positions that should be written with the content state.
    pub fn stored_indexes(&self) -> (usize, usize) {
        match self.mode {
            PracticeMode::Sentence => (self.item_index, self.word_index),
            PracticeMode::Word => (self.sentence_index, self.item_index),
        }
    }

    /// Return a snapshot without changing the current practice text.
    pub fn snapshot(
        &mut self,
        catalog: &ContentCatalog,
        milliseconds: u64,
    ) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        self.make_snapshot(catalog, milliseconds)
    }

    /// Apply user input to the Rust-owned practice state and return a snapshot.
    pub fn update(
        &mut self,
        catalog: &ContentCatalog,
        value: &str,
        milliseconds: u64,
    ) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        let item = catalog.current_item(self.mode, &self.collection_id, self.item_index)?;
        self.practice = update_practice(&self.practice, value, &item.target);
        self.make_snapshot(catalog, milliseconds)
    }

    /// Reset the current practice item without changing collection selection.
    pub fn reset(&mut self, catalog: &ContentCatalog) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        self.practice = Practice::default();
        self.make_snapshot(catalog, 0)
    }

    /// Advance after completion and return the new empty snapshot.
    pub fn advance(&mut self, catalog: &ContentCatalog) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        let current = self.make_snapshot(catalog, 0)?;
        if !current.stats.done && !attempt_reaches_end(&current.practice, &current.target) {
            return Err(ContentError::ItemNotComplete);
        }
        if self.mode == PracticeMode::Word && current.is_last_item {
            return Err(ContentError::EndOfCollection);
        }
        self.item_index = (self.item_index + 1) % current.item_count;
        self.practice = Practice::default();
        self.remember_position();
        self.make_snapshot(catalog, 0)
    }

    /// Select a word chapter and reset its first word.
    pub fn select_chapter(
        &mut self,
        catalog: &ContentCatalog,
        chapter: usize,
    ) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        self.item_index = catalog.chapter_start(self.mode, &self.collection_id, chapter)?;
        self.practice = Practice::default();
        self.remember_position();
        self.make_snapshot(catalog, 0)
    }

    fn sync_selection(&mut self, catalog: &ContentCatalog) -> Result<(), ContentError> {
        let mode = catalog.mode();
        let collection_id = catalog.selected_collection_id().to_owned();
        if self.mode != mode {
            self.remember_position();
            self.mode = mode;
            self.collection_id = collection_id;
            self.item_index = match mode {
                PracticeMode::Sentence => self.sentence_index,
                PracticeMode::Word => self.word_index,
            };
            self.practice = Practice::default();
        } else if self.collection_id != collection_id {
            self.collection_id = collection_id;
            self.item_index = 0;
            self.practice = Practice::default();
            self.remember_position();
        }
        let count = catalog.item_count(self.mode, &self.collection_id)?;
        if count == 0 {
            return Err(ContentError::UnknownCollection(self.collection_id.clone()));
        }
        if self.item_index >= count {
            self.item_index = 0;
            self.practice = Practice::default();
        }
        self.remember_position();
        Ok(())
    }

    fn remember_position(&mut self) {
        match self.mode {
            PracticeMode::Sentence => self.sentence_index = self.item_index,
            PracticeMode::Word => self.word_index = self.item_index,
        }
    }

    fn make_snapshot(
        &self,
        catalog: &ContentCatalog,
        milliseconds: u64,
    ) -> Result<PracticeSnapshot, ContentError> {
        let item = catalog.current_item(self.mode, &self.collection_id, self.item_index)?;
        let stats = statistics(&self.practice, &item.target, milliseconds);
        let completed_items = self.item_index - item.chapter_start + usize::from(stats.done);
        let chapter_progress = if self.mode == PracticeMode::Sentence && item.chapter_count == 1 {
            stats.percent
        } else {
            ((completed_items * 100) / item.chapter_len.max(1)) as u32
        };
        let chapter_index = item.chapter_index + 1;
        let chapter_count = item.chapter_count;
        let chapter_item_index = self.item_index - item.chapter_start + 1;
        let chapter_item_count = item.chapter_len;
        let is_chapter_last_item = chapter_item_index == chapter_item_count;
        let is_last_item = self.item_index + 1 == item.item_count;
        Ok(PracticeSnapshot {
            mode: self.mode,
            collection_id: self.collection_id.clone(),
            collection_title: item.collection_title,
            item_index: self.item_index + 1,
            item_count: item.item_count,
            chapter_index,
            chapter_count,
            chapter_item_index,
            chapter_item_count,
            chapter_progress: chapter_progress.min(100),
            is_chapter_last_item,
            is_last_item,
            target: item.target.clone(),
            word: item.word,
            chapter_title: item.chapter_title,
            chapter_titles: item.chapter_titles,
            translation: item.translation,
            stats,
            practice: self.practice.clone(),
        })
    }
}

fn chapter_count(item_count: usize) -> usize {
    item_count.div_ceil(CHAPTER_LENGTH).max(1)
}

fn attempt_reaches_end(practice: &Practice, target: &str) -> bool {
    let typed = practice.text.nfc().count();
    let expected = target.nfc().count();
    expected > 0 && typed >= expected
}

fn article_chapter_count(article: &Article) -> usize {
    if article.chapter_ends.is_empty() {
        1
    } else {
        article.chapter_ends.len()
    }
}

struct ArticleSpan {
    index: usize,
    count: usize,
    start: usize,
    len: usize,
    title: String,
}

fn article_span(article: &Article, item_index: usize) -> ArticleSpan {
    if article.chapter_ends.is_empty() {
        return ArticleSpan {
            index: 0,
            count: 1,
            start: 0,
            len: article.sentences.len().max(1),
            title: String::new(),
        };
    }
    let count = article.chapter_ends.len();
    let mut start = 0;
    for (index, end) in article.chapter_ends.iter().copied().enumerate() {
        let end = end.min(article.sentences.len());
        if item_index < end || index + 1 == count {
            return ArticleSpan {
                index,
                count,
                start,
                len: end.saturating_sub(start).max(1),
                title: article
                    .chapter_titles
                    .get(index)
                    .cloned()
                    .unwrap_or_default(),
            };
        }
        start = end;
    }
    ArticleSpan {
        index: 0,
        count: 1,
        start: 0,
        len: article.sentences.len().max(1),
        title: String::new(),
    }
}

fn clean_article_text(text: &str) -> String {
    let normalized: String = text.nfc().collect();
    let normalized = normalized.strip_prefix('\u{feff}').unwrap_or(&normalized);
    normalized
        .chars()
        .map(|character| match character {
            '\r' | '\n' => ' ',
            other => other,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_sentences(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let is_terminal = matches!(character, '!' | '?' | '。' | '！' | '？')
            || (character == '.' && period_ends_sentence(&characters, index, start));
        if !is_terminal {
            index += 1;
            continue;
        }
        while index + 1 < characters.len()
            && matches!(characters[index + 1], '!' | '?' | '.' | '。' | '！' | '？')
        {
            index += 1;
        }
        while index + 1 < characters.len()
            && matches!(
                characters[index + 1],
                '"' | '\'' | '”' | '’' | '」' | '』' | '）' | ')' | ']' | '}'
            )
        {
            index += 1;
        }
        let sentence: String = characters[start..=index].iter().collect();
        if !sentence.trim().is_empty() {
            sentences.push(sentence.trim().to_owned());
        }
        start = index + 1;
        while start < characters.len() && characters[start].is_whitespace() {
            start += 1;
        }
        index = start;
    }
    if start < characters.len() {
        let trailing: String = characters[start..].iter().collect();
        if !trailing.trim().is_empty() {
            sentences.push(trailing.trim().to_owned());
        }
    }
    sentences
}

fn period_ends_sentence(characters: &[char], index: usize, start: usize) -> bool {
    let previous = index
        .checked_sub(1)
        .and_then(|position| characters.get(position));
    let next = characters.get(index + 1).copied();
    if matches!((previous, next), (Some(left), Some(right)) if left.is_ascii_digit() && right.is_ascii_digit())
    {
        return false;
    }
    if matches!(next, Some('.') | Some('!') | Some('?')) {
        return false;
    }
    if next.is_some_and(|character| {
        !character.is_whitespace() && !matches!(character, '"' | '\'' | '”' | '’' | '」' | '』')
    }) {
        return false;
    }
    let token_start = characters[..index]
        .iter()
        .rposition(|character| character.is_whitespace())
        .map_or(start, |position| position + 1);
    let token: String = characters[token_start..index].iter().collect();
    !matches!(
        token.as_str(),
        "Mr" | "Mrs" | "Ms" | "Dr" | "Prof" | "Sr" | "Jr" | "St" | "vs" | "e.g" | "i.e" | "etc"
    )
}

fn stable_id(prefix: &str, title: &str, body: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in prefix.bytes().chain(title.bytes()).chain(body.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}:{hash:016x}")
}

fn builtin_articles() -> Result<Vec<Article>, ContentError> {
    Ok(vec![movie_article()?])
}

#[derive(Deserialize)]
struct MovieLineBook {
    title: String,
    source: String,
    chapters: Vec<MovieChapter>,
}

#[derive(Deserialize)]
struct MovieChapter {
    title: String,
    lines: Vec<MovieLine>,
}

#[derive(Deserialize)]
struct MovieLine {
    en: String,
    zh: String,
}

fn movie_article() -> Result<Article, ContentError> {
    let book: MovieLineBook = serde_json::from_str(include_str!("../data/movie_lines.json"))
        .map_err(|error| ContentError::BuiltinArticle(error.to_string()))?;
    let mut sentences = Vec::new();
    let mut translations = Vec::new();
    let mut chapter_titles = Vec::new();
    let mut chapter_ends = Vec::new();
    for chapter in book.chapters {
        let lines: Vec<MovieLine> = chapter
            .lines
            .into_iter()
            .filter(|line| !line.en.trim().is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        chapter_titles.push(chapter.title);
        for line in lines {
            sentences.push(normalize_practice_text(line.en.trim()));
            translations.push(line.zh.trim().to_owned());
        }
        chapter_ends.push(sentences.len());
    }
    if sentences.is_empty() || translations.len() != sentences.len() {
        return Err(ContentError::BuiltinArticle("没有可练习的台词".to_owned()));
    }
    Ok(Article {
        id: "builtin:movie-lines".to_owned(),
        title: if book.title.trim().is_empty() {
            "电影台词".to_owned()
        } else {
            book.title
        },
        source: book.source,
        sentences,
        translations,
        chapter_titles,
        chapter_ends,
    })
}

fn normalize_practice_text(text: &str) -> String {
    let mut normalized = String::new();
    for character in text.chars() {
        match character {
            '\u{2018}' | '\u{2019}' | '\u{2032}' => normalized.push('\''),
            '\u{201c}' | '\u{201d}' => normalized.push('"'),
            '\u{2013}' | '\u{2014}' => normalized.push('-'),
            '\u{2026}' => normalized.push_str("..."),
            '\u{00a0}' => normalized.push(' '),
            other => normalized.push(other),
        }
    }
    normalized
}

fn builtin_word_book() -> Result<WordBook, ContentError> {
    static BOOK: OnceLock<Result<WordBook, ContentError>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let words = serde_json::from_str::<Vec<WordEntry>>(include_str!("../data/cet4_3.json"))
            .map_err(|error| ContentError::BuiltinWordBook(error.to_string()))?;
        if words.len() != 2607 {
            return Err(ContentError::BuiltinWordBook(format!(
                "期望 2607 条，实际 {} 条",
                words.len()
            )));
        }
        Ok(WordBook {
            id: "builtin:cet4_3".to_owned(),
            title: "CET4_3 · 2607 词".to_owned(),
            source: "kajweb/dict；数据来自有道/新东方；仅作本机练习，不原样再发布词书".to_owned(),
            words,
        })
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backspace_retains_previous_mistakes() {
        let wrong = update_practice(&Practice::default(), "ax", "abc");
        let shortened = update_practice(&wrong, "a", "abc");
        let finished = update_practice(&shortened, "abc", "abc");
        assert_eq!(statistics(&finished, "abc", 60_000).accuracy, 75);
        assert_eq!(statistics(&finished, "abc", 60_000).cpm, 3);
    }

    #[test]
    fn unicode_scalars_are_counted_as_characters() {
        let practice = update_practice(&Practice::default(), "中文𠮷", "中文𠮷");
        assert_eq!(practice.attempts, 3);
        assert!(statistics(&practice, "中文𠮷", 60_000).done);
    }

    #[test]
    fn article_parser_normalizes_paragraphs_and_handles_abbreviations_and_quotes() {
        let article = parse_article(
            "sample",
            "test",
            "Dr. Smith said, \"Use 3.14 now!\"\n\nNext? Final.",
        )
        .expect("test input is valid");
        assert_eq!(
            article.sentences,
            ["Dr. Smith said, \"Use 3.14 now!\"", "Next?", "Final."]
        );
    }

    #[test]
    fn article_without_terminal_punctuation_is_one_sentence() {
        let article = parse_article("sample", "test", "one\n two").expect("test input is valid");
        assert_eq!(article.sentences, ["one two"]);
    }

    #[test]
    fn word_table_validates_header_and_keeps_first_duplicate() {
        let (book, report) = parse_word_book(
            "words",
            "test",
            "name\ttrans\tusphone\tukphone\nalpha\t一；二\tA\tB\nalpha\t替代\tC\tD\nbeta\t\t\t\n",
        )
        .expect("test table is valid");
        assert_eq!(book.words.len(), 2);
        assert_eq!(book.words[0].trans, ["一", "二"]);
        assert_eq!(book.words[0].usphone, "A");
        assert_eq!(report.duplicate_count, 1);
    }

    #[test]
    fn builtin_cet4_book_has_2607_flat_records() {
        let catalog = ContentCatalog::new().expect("embedded content is valid");
        let snapshot = catalog.snapshot();
        assert_eq!(snapshot.word_books[0].count, 2607);
    }

    #[test]
    fn session_advances_only_after_completion() {
        let catalog = ContentCatalog::new().expect("embedded content is valid");
        let mut session = PracticeSession::new(&catalog).expect("embedded content is valid");
        assert_eq!(
            session.advance(&catalog),
            Err(ContentError::ItemNotComplete)
        );
        let target = session
            .snapshot(&catalog, 0)
            .expect("embedded content is valid")
            .target;
        let completed = session
            .update(&catalog, &target, 60_000)
            .expect("embedded content is valid");
        assert!(completed.stats.done);
        let next = session.advance(&catalog).expect("completed item advances");
        assert_eq!(next.item_index, 2);
        assert!(next.practice.text.is_empty());

        let mut skipped = PracticeSession::new(&catalog).expect("embedded content is valid");
        let target = skipped
            .snapshot(&catalog, 0)
            .expect("embedded content is valid")
            .target;
        let short = target.chars().take(target.chars().count().saturating_sub(1)).collect::<String>();
        skipped
            .update(&catalog, &short, 1_000)
            .expect("short attempt can be stored");
        assert_eq!(skipped.advance(&catalog), Err(ContentError::ItemNotComplete));
        let same_length = "x".repeat(target.chars().count());
        let wrong = skipped
            .update(&catalog, &same_length, 1_000)
            .expect("full-length attempt can be stored");
        assert!(!wrong.stats.done);
        let moved = skipped.advance(&catalog).expect("reaching the end can advance");
        assert_eq!(moved.item_index, 2);
    }

    #[test]
    fn word_chapters_are_twenty_items_and_report_chapter_progress() {
        let mut catalog = ContentCatalog::new().expect("embedded content is valid");
        catalog.set_mode(PracticeMode::Word);
        let mut session = PracticeSession::new(&catalog).expect("embedded content is valid");
        let first = session
            .snapshot(&catalog, 0)
            .expect("embedded content is valid");
        assert_eq!(first.chapter_index, 1);
        assert_eq!(first.chapter_count, 131);
        assert_eq!(first.chapter_item_index, 1);
        assert_eq!(first.chapter_item_count, 20);
        assert_eq!(first.chapter_progress, 0);

        let selected = session
            .select_chapter(&catalog, 1)
            .expect("second chapter exists");
        assert_eq!(selected.item_index, 21);
        assert_eq!(selected.chapter_index, 2);
        assert_eq!(selected.chapter_item_index, 1);

        let mut last = session
            .select_chapter(&catalog, 130)
            .expect("last chapter exists");
        assert_eq!(last.chapter_item_count, 7);
        for _ in 0..6 {
            let completed = session
                .update(&catalog, &last.target, 60_000)
                .expect("chapter word can be completed");
            assert!(completed.stats.done);
            last = session.advance(&catalog).expect("next chapter word exists");
        }
        assert_eq!(last.chapter_item_index, 7);
        assert!(last.is_chapter_last_item);
        assert!(last.is_last_item);
        let completed = session
            .update(&catalog, &last.target, 60_000)
            .expect("last word can be completed");
        assert_eq!(completed.chapter_progress, 100);
        assert_eq!(
            session.advance(&catalog),
            Err(ContentError::EndOfCollection)
        );
    }

    #[test]
    fn stored_progress_restores_each_mode() {
        let mut catalog = ContentCatalog::new().expect("embedded content is valid");
        catalog.set_mode(PracticeMode::Word);
        let mut session = PracticeSession::with_stored_progress(&catalog, 1, 20)
            .expect("stored positions are valid");
        let word = session
            .snapshot(&catalog, 0)
            .expect("word position restores");
        assert_eq!(word.item_index, 21);
        assert_eq!(word.chapter_index, 2);

        catalog.set_mode(PracticeMode::Sentence);
        let sentence = session
            .snapshot(&catalog, 0)
            .expect("sentence position restores");
        assert_eq!(sentence.item_index, 2);
        assert_eq!(session.stored_indexes(), (1, 20));
    }

    #[test]
    fn invalid_word_row_names_its_line_and_empty_article_is_rejected() {
        let error = parse_word_book("words", "test", "name\ttrans\nok\t一\n\t有释义")
            .expect_err("blank name is invalid");
        assert_eq!(
            error,
            ContentError::InvalidWordRow {
                line: 3,
                reason: "name 不能为空".to_owned(),
            }
        );
        assert_eq!(error.to_string(), "第 3 行无效：name 不能为空");
        assert_eq!(
            parse_article("empty", "test", " \n\t").expect_err("blank article is invalid"),
            ContentError::EmptyArticle
        );
    }

    #[test]
    fn sentence_chapters_follow_declared_boundaries() {
        let mut catalog = ContentCatalog::new().expect("embedded content is valid");
        catalog.add_article(Article {
            id: "test:movies".to_owned(),
            title: "电影台词".to_owned(),
            source: "test".to_owned(),
            sentences: vec!["One.".to_owned(), "Two.".to_owned(), "Three.".to_owned()],
            translations: vec!["一".to_owned(), "二".to_owned(), "三".to_owned()],
            chapter_titles: vec!["Gone with the Wind".to_owned(), "Forrest Gump".to_owned()],
            chapter_ends: vec![2, 3],
        });
        let mut session = PracticeSession::new(&catalog).expect("article can be practiced");
        let first = session.snapshot(&catalog, 0).expect("first line exists");
        assert_eq!(first.chapter_index, 1);
        assert_eq!(first.chapter_count, 2);
        assert_eq!(first.chapter_title, "Gone with the Wind");
        assert_eq!(first.chapter_titles.len(), 2);
        assert_eq!(first.chapter_item_count, 2);
        assert_eq!(first.translation, "一");
        assert_eq!(
            catalog
                .snapshot()
                .articles
                .iter()
                .find(|item| item.id == "test:movies")
                .map(|item| item.chapter_count),
            Some(2)
        );

        let completed = session
            .update(&catalog, &first.target, 1_000)
            .expect("line can be completed");
        assert!(completed.stats.done);
        let second = session.advance(&catalog).expect("second line exists");
        assert_eq!(second.chapter_item_index, 2);
        assert!(second.is_chapter_last_item);
        assert!(!second.is_last_item);

        let next_movie = session
            .select_chapter(&catalog, 1)
            .expect("second chapter exists");
        assert_eq!(next_movie.target, "Three.");
        assert_eq!(next_movie.chapter_title, "Forrest Gump");
        assert_eq!(next_movie.translation, "三");
        assert_eq!(next_movie.chapter_item_index, 1);
        assert_eq!(next_movie.chapter_item_count, 1);
    }

    #[test]
    fn builtin_movie_lines_keep_movie_chapters_and_typeable_punctuation() {
        let catalog = ContentCatalog::new().expect("embedded content is valid");
        let articles = catalog.snapshot().articles;
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].id, "builtin:movie-lines");
        assert_eq!(articles[0].title, "电影台词");
        assert_eq!(articles[0].count, 79);
        assert_eq!(articles[0].chapter_count, 8);
        assert!(!articles[0].removable);

        let mut session = PracticeSession::new(&catalog).expect("movie lines can be practiced");
        let first = session.snapshot(&catalog, 0).expect("first line exists");
        assert_eq!(first.chapter_title, "乱世佳人");
        assert_eq!(first.chapter_item_count, 11);
        assert_eq!(first.translation, "不管怎样，明天是新的一天!");

        let typed = PracticeSession::with_stored_progress(&catalog, 3, 0)
            .expect("fourth line exists")
            .snapshot(&catalog, 0)
            .expect("fourth line exists");
        assert!(typed.target.contains("it's"));
        assert!(!typed.target.chars().any(|character| matches!(
            character,
            '\u{2018}' | '\u{2019}' | '\u{2014}' | '\u{2026}'
        )));

        let second = session
            .select_chapter(&catalog, 1)
            .expect("second movie exists");
        assert_eq!(second.chapter_title, "怦然心动");
        assert_eq!(second.chapter_index, 2);
        assert_eq!(second.item_index, 12);
    }
}
