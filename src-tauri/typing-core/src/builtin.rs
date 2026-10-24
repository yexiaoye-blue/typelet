//! Sentence and word collections shipped with the application.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::model::{Article, ContentError, WordBook, WordEntry};

/// Sentence collections shipped with the application.
pub(crate) fn builtin_articles() -> Result<Vec<Article>, ContentError> {
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

/// Built-in CET-4 word book.
pub(crate) fn builtin_word_book() -> Result<WordBook, ContentError> {
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
