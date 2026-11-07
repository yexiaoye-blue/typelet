//! Attempt accounting and the statistics shown while typing.

use unicode_normalization::UnicodeNormalization;

use crate::model::{Practice, Statistics};

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
