//! Owned articles, word books, and the current selection.

use std::collections::BTreeMap;

use crate::builtin::{builtin_articles, builtin_word_book};
use crate::chapter::{
    numbered_chapter_count, numbered_chapter_start, numbered_span, presented_index,
    progress_percent, PracticeLayout, DEFAULT_CHAPTER_LENGTH,
};
use crate::model::{
    Article, CollectionSummary, ContentCatalogSnapshot, ContentError, PracticeItem, PracticeMode,
    StoredContentState, WordBook,
};
use crate::parse::article_span;

/// Store the content collections and the last selected collection.
pub struct ContentCatalog {
    builtin_articles: Vec<Article>,
    builtin_word_book: WordBook,
    articles: Vec<Article>,
    word_books: Vec<WordBook>,
    mode: PracticeMode,
    /// Id of the article selected for sentence practice.
    pub(crate) article_id: String,
    /// Id of the word book selected for word practice.
    pub(crate) word_book_id: String,
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
            positions: BTreeMap::new(),
        }
    }

    /// Return a compact selector snapshot.
    pub fn snapshot(&self) -> ContentCatalogSnapshot {
        self.snapshot_with_positions(&BTreeMap::new(), DEFAULT_CHAPTER_LENGTH)
    }

    /// Return a selector snapshot with saved progress for each collection.
    ///
    /// `chapter_length` changes numbered chapter counts only. Progress stays the
    /// saved cursor divided by the collection size.
    pub fn snapshot_with_positions(
        &self,
        positions: &BTreeMap<String, usize>,
        chapter_length: usize,
    ) -> ContentCatalogSnapshot {
        let articles = self
            .builtin_articles
            .iter()
            .map(|article| self.article_summary(article, false, positions, chapter_length))
            .chain(
                self.articles
                    .iter()
                    .map(|article| self.article_summary(article, true, positions, chapter_length)),
            )
            .collect();
        let word_books = std::iter::once(self.word_book_summary(
            &self.builtin_word_book,
            false,
            positions,
            chapter_length,
        ))
        .chain(
            self.word_books
                .iter()
                .map(|book| self.word_book_summary(book, true, positions, chapter_length)),
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

    fn article_summary(
        &self,
        article: &Article,
        removable: bool,
        positions: &BTreeMap<String, usize>,
        chapter_length: usize,
    ) -> CollectionSummary {
        CollectionSummary {
            id: article.id.clone(),
            title: article.title.clone(),
            count: article.sentences.len(),
            selected: self.article_id == article.id,
            source: article.source.clone(),
            chapter_count: if article.chapter_ends.is_empty() {
                numbered_chapter_count(article.sentences.len(), chapter_length)
            } else {
                article.chapter_ends.len()
            },
            progress: progress_percent(
                positions.get(&article.id).copied().unwrap_or(0),
                article.sentences.len(),
            ),
            removable,
        }
    }

    fn word_book_summary(
        &self,
        book: &WordBook,
        removable: bool,
        positions: &BTreeMap<String, usize>,
        chapter_length: usize,
    ) -> CollectionSummary {
        CollectionSummary {
            id: book.id.clone(),
            title: book.title.clone(),
            count: book.words.len(),
            selected: self.word_book_id == book.id,
            source: book.source.clone(),
            chapter_count: numbered_chapter_count(book.words.len(), chapter_length),
            progress: progress_percent(
                positions.get(&book.id).copied().unwrap_or(0),
                book.words.len(),
            ),
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

    /// Id of the collection selected for the active mode.
    pub(crate) fn selected_collection_id(&self) -> &str {
        match self.mode {
            PracticeMode::Sentence => &self.article_id,
            PracticeMode::Word => &self.word_book_id,
        }
    }

    /// Number of practice items in one collection.
    pub(crate) fn item_count(&self, mode: PracticeMode, id: &str) -> Result<usize, ContentError> {
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

    /// Presented item for a saved cursor, including chapter order.
    pub(crate) fn current_item(
        &self,
        mode: PracticeMode,
        id: &str,
        index: usize,
        layout: PracticeLayout,
    ) -> Result<PracticeItem, ContentError> {
        match mode {
            PracticeMode::Sentence => {
                let article = self
                    .article(id)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                let authored = !article.chapter_ends.is_empty();
                let span = if authored {
                    article_span(article, index)
                } else {
                    numbered_span(index, article.sentences.len(), layout.chapter_length)
                };
                let content_index =
                    presented_index(&article.id, span.start, span.len, index, layout.order);
                let target = article
                    .sentences
                    .get(content_index)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                Ok(PracticeItem {
                    collection_title: article.title.clone(),
                    item_count: article.sentences.len(),
                    target: target.clone(),
                    word: None,
                    translation: article
                        .translations
                        .get(content_index)
                        .cloned()
                        .unwrap_or_default(),
                    chapter_title: span.title,
                    chapter_titles: if authored {
                        article.chapter_titles.clone()
                    } else {
                        Vec::new()
                    },
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
                let span = numbered_span(index, book.words.len(), layout.chapter_length);
                let content_index =
                    presented_index(&book.id, span.start, span.len, index, layout.order);
                let word = book
                    .words
                    .get(content_index)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                Ok(PracticeItem {
                    collection_title: book.title.clone(),
                    item_count: book.words.len(),
                    target: word.name.clone(),
                    word: Some(word.clone()),
                    translation: String::new(),
                    chapter_title: String::new(),
                    chapter_titles: Vec::new(),
                    chapter_index: span.index,
                    chapter_count: span.count,
                    chapter_start: span.start,
                    chapter_len: span.len,
                })
            }
        }
    }

    /// First item index of a chapter in the selected collection.
    pub(crate) fn chapter_start(
        &self,
        mode: PracticeMode,
        id: &str,
        chapter: usize,
        layout: PracticeLayout,
    ) -> Result<usize, ContentError> {
        match mode {
            PracticeMode::Word => {
                numbered_chapter_start(self.item_count(mode, id)?, chapter, layout.chapter_length)
            }
            PracticeMode::Sentence => {
                let article = self
                    .article(id)
                    .ok_or_else(|| ContentError::UnknownCollection(id.to_owned()))?;
                if article.chapter_ends.is_empty() {
                    return numbered_chapter_start(
                        article.sentences.len(),
                        chapter,
                        layout.chapter_length,
                    );
                }
                let chapters = article.chapter_ends.len();
                if chapter >= chapters {
                    return Err(ContentError::InvalidChapter {
                        chapter,
                        count: chapters,
                    });
                }
                if chapter == 0 {
                    Ok(0)
                } else {
                    Ok(article.chapter_ends[chapter - 1])
                }
            }
        }
    }
}
