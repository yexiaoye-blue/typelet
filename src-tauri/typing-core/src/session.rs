//! Current item, saved progress, and typing counters.

use std::collections::BTreeMap;

use crate::catalog::ContentCatalog;
use crate::chapter::{attempt_reaches_end, PracticeLayout};
use crate::model::{ContentError, Practice, PracticeMode, PracticeOrder, PracticeSnapshot};
use crate::scoring::{statistics, update_practice};

/// Rust-owned current item, position, and typing counters.
#[derive(Clone, Debug)]
pub struct PracticeSession {
    mode: PracticeMode,
    collection_id: String,
    item_index: usize,
    sentence_index: usize,
    word_index: usize,
    positions: BTreeMap<String, usize>,
    practice: Practice,
    layout: PracticeLayout,
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
            positions: BTreeMap::new(),
            practice: Practice::default(),
            layout: PracticeLayout::default(),
        })
    }

    /// Resume a saved sentence and word position for the current catalog selection.
    pub fn with_stored_progress(
        catalog: &ContentCatalog,
        sentence_index: usize,
        word_index: usize,
    ) -> Result<Self, ContentError> {
        Self::with_positions(catalog, sentence_index, word_index, BTreeMap::new())
    }

    /// Resume saved positions for every collection.
    ///
    /// The sentence and word indexes belong to the catalog's selected article and word book.
    /// Indexes for other collections come from positions.
    pub fn with_positions(
        catalog: &ContentCatalog,
        sentence_index: usize,
        word_index: usize,
        mut positions: BTreeMap<String, usize>,
    ) -> Result<Self, ContentError> {
        let mut session = Self::new(catalog)?;
        positions.insert(catalog.article_id.clone(), sentence_index);
        positions.insert(catalog.word_book_id.clone(), word_index);
        session.positions = positions;
        session.sentence_index = sentence_index;
        session.word_index = word_index;
        session.item_index = match catalog.mode() {
            PracticeMode::Sentence => sentence_index,
            PracticeMode::Word => word_index,
        };
        session.sync_selection(catalog)?;
        Ok(session)
    }

    /// Saved resume position for every collection this session has opened.
    pub fn positions(&self) -> &BTreeMap<String, usize> {
        &self.positions
    }

    /// Requested chapter size before the open collection clamps it.
    pub fn chapter_length(&self) -> usize {
        self.layout.chapter_length
    }

    /// Change numbered chapter size and the order inside a chapter.
    ///
    /// The saved cursor stays where it is, so library progress does not move.
    pub fn set_layout(
        &mut self,
        catalog: &ContentCatalog,
        chapter_length: usize,
        order: PracticeOrder,
    ) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        let previous =
            catalog.current_item(self.mode, &self.collection_id, self.item_index, self.layout)?;
        self.layout = PracticeLayout::new(chapter_length, order);
        let next =
            catalog.current_item(self.mode, &self.collection_id, self.item_index, self.layout)?;
        if next.target != previous.target {
            self.practice = Practice::default();
        }
        self.make_snapshot(catalog, 0)
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
        let item =
            catalog.current_item(self.mode, &self.collection_id, self.item_index, self.layout)?;
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

    /// Select an item in the current collection and reset its practice text.
    pub fn select_item(
        &mut self,
        catalog: &ContentCatalog,
        item: usize,
    ) -> Result<PracticeSnapshot, ContentError> {
        self.sync_selection(catalog)?;
        let count = catalog.item_count(self.mode, &self.collection_id)?;
        if item >= count {
            return Err(ContentError::InvalidItem { item, count });
        }
        self.item_index = item;
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
        self.item_index =
            catalog.chapter_start(self.mode, &self.collection_id, chapter, self.layout)?;
        self.practice = Practice::default();
        self.remember_position();
        self.make_snapshot(catalog, 0)
    }

    fn sync_selection(&mut self, catalog: &ContentCatalog) -> Result<(), ContentError> {
        let mode = catalog.mode();
        let collection_id = catalog.selected_collection_id().to_owned();
        if self.mode != mode || self.collection_id != collection_id {
            self.remember_position();
            self.mode = mode;
            self.collection_id = collection_id;
            self.item_index = self
                .positions
                .get(&self.collection_id)
                .copied()
                .unwrap_or(0);
            self.practice = Practice::default();
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
        if !self.collection_id.is_empty() {
            self.positions
                .insert(self.collection_id.clone(), self.item_index);
        }
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
        let item =
            catalog.current_item(self.mode, &self.collection_id, self.item_index, self.layout)?;
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
