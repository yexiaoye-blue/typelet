//! Chapter size and the stable shuffle used inside one chapter.

use unicode_normalization::UnicodeNormalization;

use crate::model::{ArticleSpan, ContentError, Practice, PracticeOrder};

/// Default numbered chapter size before the open collection clamps it.
pub(crate) const DEFAULT_CHAPTER_LENGTH: usize = 20;
const MIN_CHAPTER_LENGTH: usize = 10;

/// Requested chapter size and the order used inside a chapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PracticeLayout {
    /// Requested numbered-chapter size after the minimum is applied.
    pub(crate) chapter_length: usize,
    /// Order used to present items inside the current chapter.
    pub(crate) order: PracticeOrder,
}

impl Default for PracticeLayout {
    fn default() -> Self {
        Self {
            chapter_length: DEFAULT_CHAPTER_LENGTH,
            order: PracticeOrder::Book,
        }
    }
}

impl PracticeLayout {
    /// Raise a zero size to the default and enforce the minimum size.
    pub(crate) fn new(chapter_length: usize, order: PracticeOrder) -> Self {
        Self {
            chapter_length: normalize_requested_chapter_length(chapter_length),
            order,
        }
    }
}

fn normalize_requested_chapter_length(value: usize) -> usize {
    if value == 0 {
        DEFAULT_CHAPTER_LENGTH
    } else {
        value.max(MIN_CHAPTER_LENGTH)
    }
}

fn effective_chapter_length(requested: usize, count: usize) -> usize {
    let requested = normalize_requested_chapter_length(requested);
    if count == 0 {
        return requested;
    }
    let min = MIN_CHAPTER_LENGTH.min(count);
    requested.clamp(min, count)
}

/// How many numbered chapters this collection size produces.
pub(crate) fn numbered_chapter_count(item_count: usize, requested: usize) -> usize {
    if item_count == 0 {
        return 1;
    }
    item_count
        .div_ceil(effective_chapter_length(requested, item_count))
        .max(1)
}

/// First item index of a numbered chapter, or an error when the chapter is out of range.
pub(crate) fn numbered_chapter_start(
    item_count: usize,
    chapter: usize,
    requested: usize,
) -> Result<usize, ContentError> {
    let chapters = numbered_chapter_count(item_count, requested);
    if chapter >= chapters {
        return Err(ContentError::InvalidChapter {
            chapter,
            count: chapters,
        });
    }
    Ok(chapter * effective_chapter_length(requested, item_count))
}

/// Numbered chapter window that contains a saved cursor.
pub(crate) fn numbered_span(cursor: usize, item_count: usize, requested: usize) -> ArticleSpan {
    let length = effective_chapter_length(requested, item_count.max(1));
    let chapter = if item_count == 0 { 0 } else { cursor / length };
    let start = chapter.saturating_mul(length);
    let end = start.saturating_add(length).min(item_count);
    ArticleSpan {
        index: chapter,
        count: numbered_chapter_count(item_count, requested),
        start,
        len: end.saturating_sub(start).max(1),
        title: String::new(),
    }
}

/// Stable order for one chapter. The saved cursor is not part of the seed, so
/// shuffling cannot move a collection's progress position.
pub(crate) fn chapter_offsets(
    collection_id: &str,
    chapter_start: usize,
    chapter_len: usize,
) -> Vec<usize> {
    let mut offsets: Vec<usize> = (0..chapter_len).collect();
    if chapter_len < 2 {
        return offsets;
    }
    let mut state = hash32(collection_id)
        ^ (chapter_start as u32).wrapping_mul(0x9E37_79B1)
        ^ (chapter_len as u32).wrapping_mul(0x85EB_CA77);
    if state == 0 {
        state = 0xA5A5_A5A5;
    }
    for index in (1..chapter_len).rev() {
        state = mix32(state);
        let swap = state as usize % (index + 1);
        offsets.swap(index, swap);
    }
    offsets
}

fn hash32(id: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn mix32(mut value: u32) -> u32 {
    value ^= value.wrapping_shl(13);
    value ^= value >> 17;
    value ^= value.wrapping_shl(5);
    value
}

/// Collection index shown for a saved cursor under the active chapter order.
pub(crate) fn presented_index(
    collection_id: &str,
    chapter_start: usize,
    chapter_len: usize,
    cursor: usize,
    order: PracticeOrder,
) -> usize {
    let slot = cursor.saturating_sub(chapter_start);
    if order != PracticeOrder::Random || chapter_len < 2 {
        return chapter_start + slot.min(chapter_len.saturating_sub(1));
    }
    let offsets = chapter_offsets(collection_id, chapter_start, chapter_len);
    chapter_start + offsets.get(slot).copied().unwrap_or(0)
}

/// Percentage of a collection already passed. The index is the next item to practice.
pub(crate) fn progress_percent(index: usize, count: usize) -> u32 {
    if count == 0 || index == 0 {
        return 0;
    }
    let index = index.min(count);
    let percent = (index as u64 * 100 + count as u64 / 2) / count as u64;
    percent.clamp(1, 100) as u32
}

/// Whether the submitted text has reached the end of the target.
pub(crate) fn attempt_reaches_end(practice: &Practice, target: &str) -> bool {
    let typed = practice.text.nfc().count();
    let expected = target.nfc().count();
    expected > 0 && typed >= expected
}
