use super::*;
use crate::chapter::{chapter_offsets, progress_percent};

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
fn period_before_closing_bracket_still_ends_the_sentence() {
    let article = parse_article(
        "sample",
        "test",
        "He shouted (loudly.) Next came silence. Wow!) Truly.",
    )
    .expect("test input is valid");
    assert_eq!(
        article.sentences,
        ["He shouted (loudly.)", "Next came silence.", "Wow!)", "Truly."]
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
    let short = target
        .chars()
        .take(target.chars().count().saturating_sub(1))
        .collect::<String>();
    skipped
        .update(&catalog, &short, 1_000)
        .expect("short attempt can be stored");
    assert_eq!(
        skipped.advance(&catalog),
        Err(ContentError::ItemNotComplete)
    );
    let same_length = "x".repeat(target.chars().count());
    let wrong = skipped
        .update(&catalog, &same_length, 1_000)
        .expect("full-length attempt can be stored");
    assert!(!wrong.stats.done);
    let moved = skipped
        .advance(&catalog)
        .expect("reaching the end can advance");
    assert_eq!(moved.item_index, 2);
}

#[test]
fn session_selects_items_and_rejects_out_of_range_indexes() {
    let catalog = ContentCatalog::new().expect("embedded content is valid");
    let mut session = PracticeSession::new(&catalog).expect("embedded content is valid");
    let selected = session
        .select_item(&catalog, 1)
        .expect("second item exists");
    assert_eq!(selected.item_index, 2);
    assert_eq!(
        session.select_item(&catalog, selected.item_count),
        Err(ContentError::InvalidItem {
            item: selected.item_count,
            count: selected.item_count,
        })
    );
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
    let mut session =
        PracticeSession::with_stored_progress(&catalog, 1, 20).expect("stored positions are valid");
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
fn switching_word_books_restores_each_position_and_reports_progress() {
    let mut catalog = ContentCatalog::new().expect("embedded content is valid");
    catalog.set_mode(PracticeMode::Word);
    catalog.add_word_book(WordBook {
        id: "import:sample".to_owned(),
        title: "导入".to_owned(),
        source: "test".to_owned(),
        words: (0..40)
            .map(|index| WordEntry {
                name: format!("word{index}"),
                trans: vec!["词".to_owned()],
                usphone: String::new(),
                ukphone: String::new(),
            })
            .collect(),
    });
    let mut session = PracticeSession::new(&catalog).expect("imported book can be practiced");
    catalog
        .select_word_book("builtin:cet4_3")
        .expect("cet4 can be selected");
    let cet4 = session
        .select_item(&catalog, 20)
        .expect("cet4 word can be selected");
    assert_eq!(cet4.item_index, 21);

    catalog
        .select_word_book("import:sample")
        .expect("sample can be selected");
    let sample = session
        .snapshot(&catalog, 0)
        .expect("sample starts at the beginning");
    assert_eq!(sample.collection_id, "import:sample");
    assert_eq!(sample.item_index, 1);
    session
        .select_item(&catalog, 5)
        .expect("sample word can be selected");

    catalog
        .select_word_book("builtin:cet4_3")
        .expect("cet4 can be selected again");
    let restored = session
        .snapshot(&catalog, 0)
        .expect("cet4 position restores");
    assert_eq!(restored.item_index, 21);

    catalog
        .select_word_book("import:sample")
        .expect("sample can be selected again");
    let sample_again = session
        .snapshot(&catalog, 0)
        .expect("sample position restores");
    assert_eq!(sample_again.item_index, 6);

    let listed = catalog.snapshot_with_positions(session.positions(), session.chapter_length());
    let cet4_progress = listed
        .word_books
        .iter()
        .find(|book| book.id == "builtin:cet4_3")
        .map(|book| book.progress);
    let sample_progress = listed
        .word_books
        .iter()
        .find(|book| book.id == "import:sample")
        .map(|book| book.progress);
    assert_eq!(cet4_progress, Some(progress_percent(20, 2607)));
    assert_eq!(sample_progress, Some(13));

    let (sentence_index, word_index) = session.stored_indexes();
    let mut resumed = PracticeSession::with_positions(
        &catalog,
        sentence_index,
        word_index,
        session.positions().clone(),
    )
    .expect("saved positions can be restored");
    let resumed_snapshot = resumed
        .snapshot(&catalog, 0)
        .expect("sample is still selected");
    assert_eq!(resumed_snapshot.collection_id, "import:sample");
    assert_eq!(resumed_snapshot.item_index, 6);
    catalog
        .select_word_book("builtin:cet4_3")
        .expect("cet4 can be selected after restore");
    let resumed_cet4 = resumed
        .snapshot(&catalog, 0)
        .expect("cet4 restores from the map");
    assert_eq!(resumed_cet4.item_index, 21);
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
    assert!(!typed
        .target
        .chars()
        .any(|character| matches!(character, '\u{2018}' | '\u{2019}' | '\u{2014}' | '\u{2026}')));

    let second = session
        .select_chapter(&catalog, 1)
        .expect("second movie exists");
    assert_eq!(second.chapter_title, "怦然心动");
    assert_eq!(second.chapter_index, 2);
    assert_eq!(second.item_index, 12);
}

fn sample_words(count: usize) -> Vec<WordEntry> {
    (0..count)
        .map(|index| WordEntry {
            name: format!("word{index}"),
            trans: vec!["词".to_owned()],
            usphone: String::new(),
            ukphone: String::new(),
        })
        .collect()
}

fn chapter_targets(session: &mut PracticeSession, catalog: &ContentCatalog) -> Vec<String> {
    let mut current = session.snapshot(catalog, 0).expect("chapter can be read");
    let mut names = vec![current.target.clone()];
    while !current.is_chapter_last_item {
        current = session
            .update(catalog, &current.target, 1_000)
            .expect("item can be completed");
        assert!(current.stats.done);
        current = session.advance(catalog).expect("chapter has another item");
        names.push(current.target.clone());
    }
    names
}

#[test]
fn chapter_length_regroups_words_without_moving_the_saved_place() {
    let mut catalog = ContentCatalog::new().expect("embedded content is valid");
    catalog.set_mode(PracticeMode::Word);
    let mut session = PracticeSession::with_stored_progress(&catalog, 0, 25)
        .expect("stored word can be restored");
    let positions = session.positions().clone();
    let snapshot = session
        .set_layout(&catalog, 10, PracticeOrder::Book)
        .expect("chapter length can change");
    assert_eq!(snapshot.item_index, 26);
    assert_eq!(snapshot.chapter_index, 3);
    assert_eq!(snapshot.chapter_item_index, 6);
    assert_eq!(snapshot.chapter_item_count, 10);
    assert_eq!(session.positions(), &positions);

    let selected = session
        .select_chapter(&catalog, 1)
        .expect("second short chapter exists");
    assert_eq!(selected.item_index, 11);
    assert_eq!(selected.chapter_item_index, 1);
}

#[test]
fn chapter_length_clamps_to_the_open_collection() {
    let mut catalog = ContentCatalog::new().expect("embedded content is valid");
    catalog.set_mode(PracticeMode::Word);
    catalog.add_word_book(WordBook {
        id: "import:short".to_owned(),
        title: "短词本".to_owned(),
        source: "test".to_owned(),
        words: sample_words(12),
    });
    let mut session = PracticeSession::new(&catalog).expect("short book can be practiced");
    let whole = session
        .set_layout(&catalog, 100, PracticeOrder::Book)
        .expect("large chapter length is clamped");
    assert_eq!(whole.chapter_count, 1);
    assert_eq!(whole.chapter_item_count, 12);
    assert_eq!(session.chapter_length(), 100);

    let split = session
        .set_layout(&catalog, 5, PracticeOrder::Book)
        .expect("short chapter length is raised to the minimum");
    assert_eq!(session.chapter_length(), 10);
    assert_eq!(split.chapter_count, 2);
    assert_eq!(split.chapter_item_count, 10);
    let last = session
        .select_chapter(&catalog, 1)
        .expect("remainder chapter exists");
    assert_eq!(last.item_index, 11);
    assert_eq!(last.chapter_item_count, 2);
}

#[test]
fn random_order_shuffles_only_the_current_chapter_and_keeps_progress() {
    let mut catalog = ContentCatalog::new().expect("embedded content is valid");
    catalog.set_mode(PracticeMode::Word);
    let mut session = PracticeSession::with_stored_progress(&catalog, 0, 20)
        .expect("stored word can be restored");
    let positions = session.positions().clone();
    let book = session
        .snapshot(&catalog, 0)
        .expect("book order can be read");
    let progress = catalog
        .snapshot_with_positions(session.positions(), session.chapter_length())
        .word_books
        .into_iter()
        .find(|item| item.id == "builtin:cet4_3")
        .map(|item| item.progress);

    let random = session
        .set_layout(&catalog, 20, PracticeOrder::Random)
        .expect("random order can be selected");
    assert_eq!(random.item_index, book.item_index);
    assert_eq!(random.chapter_index, book.chapter_index);
    assert_eq!(session.positions(), &positions);
    let random_progress = catalog
        .snapshot_with_positions(session.positions(), session.chapter_length())
        .word_books
        .into_iter()
        .find(|item| item.id == "builtin:cet4_3")
        .map(|item| item.progress);
    assert_eq!(random_progress, progress);

    session
        .set_layout(&catalog, 20, PracticeOrder::Book)
        .expect("book order can be restored");
    let restored = session
        .snapshot(&catalog, 0)
        .expect("cursor can be read again");
    assert_eq!(restored.target, book.target);
    assert_eq!(restored.item_index, book.item_index);
    assert_eq!(session.positions(), &positions);

    let mut ordered = PracticeSession::new(&catalog).expect("word book can restart");
    let book_names = chapter_targets(&mut ordered, &catalog);
    let mut shuffled = PracticeSession::new(&catalog).expect("word book can restart");
    shuffled
        .set_layout(&catalog, 20, PracticeOrder::Random)
        .expect("chapter can be shuffled");
    let random_names = chapter_targets(&mut shuffled, &catalog);
    assert_eq!(book_names.len(), 20);
    assert_eq!(random_names.len(), 20);
    let mut book_sorted = book_names.clone();
    let mut random_sorted = random_names.clone();
    book_sorted.sort();
    random_sorted.sort();
    assert_eq!(random_sorted, book_sorted);
    assert_ne!(random_names, book_names);
    assert_eq!(
        chapter_offsets("builtin:cet4_3", 0, 8),
        vec![7, 0, 3, 6, 2, 5, 4, 1]
    );
}

#[test]
fn flat_sentences_follow_chapter_length_and_movies_keep_their_chapters() {
    let mut catalog = ContentCatalog::new().expect("embedded content is valid");
    catalog.add_article(Article {
        id: "test:flat".to_owned(),
        title: "平文章".to_owned(),
        source: "test".to_owned(),
        sentences: (0..25).map(|index| format!("Sentence {index}.")).collect(),
        translations: Vec::new(),
        chapter_titles: Vec::new(),
        chapter_ends: Vec::new(),
    });
    let mut session = PracticeSession::new(&catalog).expect("flat article can be practiced");
    let first = session
        .snapshot(&catalog, 0)
        .expect("flat article can be read");
    assert_eq!(first.chapter_count, 2);
    assert_eq!(first.chapter_item_count, 20);
    assert_eq!(first.target, "Sentence 0.");
    session
        .select_item(&catalog, 22)
        .expect("later sentence exists");
    let moved = session
        .set_layout(&catalog, 10, PracticeOrder::Book)
        .expect("sentence chapter length can change");
    assert_eq!(moved.item_index, 23);
    assert_eq!(moved.target, "Sentence 22.");
    assert_eq!(moved.chapter_index, 3);
    assert_eq!(moved.chapter_item_index, 3);
    assert_eq!(session.positions().get("test:flat").copied(), Some(22));

    catalog
        .select_article("builtin:movie-lines")
        .expect("movies can be selected");
    let movie = session
        .set_layout(&catalog, 10, PracticeOrder::Book)
        .expect("movie layout can be set");
    assert_eq!(movie.chapter_count, 8);
    assert_eq!(movie.chapter_item_count, 11);
    assert_eq!(movie.chapter_title, "乱世佳人");
    assert_eq!(movie.item_index, 1);

    let mut shuffled = PracticeSession::new(&catalog).expect("movies can restart");
    catalog.set_mode(PracticeMode::Sentence);
    catalog
        .select_article("builtin:movie-lines")
        .expect("movies stay selected");
    let book_names = {
        let mut ordered = PracticeSession::new(&catalog).expect("movies can restart");
        chapter_targets(&mut ordered, &catalog)
    };
    shuffled
        .set_layout(&catalog, 10, PracticeOrder::Random)
        .expect("movie chapter can be shuffled");
    let random_names = chapter_targets(&mut shuffled, &catalog);
    assert_eq!(random_names.len(), book_names.len());
    let mut book_sorted = book_names.clone();
    let mut random_sorted = random_names.clone();
    book_sorted.sort();
    random_sorted.sort();
    assert_eq!(random_sorted, book_sorted);
    let next_movie = shuffled
        .select_chapter(&catalog, 1)
        .expect("second movie exists");
    assert_eq!(next_movie.item_index, 12);
    assert_eq!(next_movie.chapter_title, "怦然心动");
    assert_eq!(
        shuffled.positions().get("builtin:movie-lines").copied(),
        Some(11)
    );
}
