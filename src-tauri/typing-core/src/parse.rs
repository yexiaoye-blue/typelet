//! Parsers for imported articles and word tables.

use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;

use crate::model::{Article, ArticleSpan, ContentError, ImportReport, WordBook, WordEntry};

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

/// Chapter window for an authored article. A flat article is one chapter.
pub(crate) fn article_span(article: &Article, item_index: usize) -> ArticleSpan {
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
