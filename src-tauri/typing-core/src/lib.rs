use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Practice {
    pub text: String,
    pub attempts: u32,
    pub correct_attempts: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    pub correct: u32,
    pub percent: u32,
    pub accuracy: u32,
    pub cpm: u32,
    pub done: bool,
}

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

pub fn statistics(practice: &Practice, target: &str, milliseconds: u64) -> Statistics {
    let typed: Vec<char> = practice.text.nfc().collect();
    let expected: Vec<char> = target.nfc().collect();
    let correct = typed
        .iter()
        .enumerate()
        .filter(|(index, character)| expected.get(*index) == Some(character))
        .count() as u32;
    let percent = if expected.is_empty() {
        0
    } else {
        correct * 100 / expected.len() as u32
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
}
