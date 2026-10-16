#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{Manager, State};
use typelet_typing::{
    parse_article, parse_word_book, ContentCatalog, ContentCatalogSnapshot, ContentError,
    ImportReport, PracticeMode, PracticeSession, PracticeSnapshot, StoredContentState,
};

mod agents;
mod fonts;

struct AppState {
    content: Mutex<ContentCatalog>,
    session: Mutex<PracticeSession>,
    storage_path: PathBuf,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    report: ImportReport,
    catalog: ContentCatalogSnapshot,
    snapshot: PracticeSnapshot,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogUpdate {
    catalog: ContentCatalogSnapshot,
    snapshot: PracticeSnapshot,
}

fn command_error(error: ContentError) -> String {
    error.to_string()
}

fn save_content(state: &AppState) -> Result<(), String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    let (sentence_index, word_index) = session.stored_indexes();
    let mut stored = catalog.stored_state();
    stored.sentence_index = sentence_index;
    stored.word_index = word_index;
    let json = serde_json::to_vec_pretty(&stored).map_err(|error| error.to_string())?;
    fs::write(&state.storage_path, json)
        .map_err(|error| format!("{}: {error}", state.storage_path.display()))
}

fn snapshot(state: &AppState, milliseconds: u64) -> Result<PracticeSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let mut session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    session
        .snapshot(&catalog, milliseconds)
        .map_err(command_error)
}

#[tauri::command]
fn get_practice_snapshot(state: State<'_, AppState>) -> Result<PracticeSnapshot, String> {
    snapshot(&state, 0)
}

#[tauri::command]
fn typing_snapshot(
    state: State<'_, AppState>,
    value: String,
    milliseconds: u64,
) -> Result<PracticeSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let mut session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    session
        .update(&catalog, &value, milliseconds)
        .map_err(command_error)
}

#[tauri::command]
fn reset_practice(state: State<'_, AppState>) -> Result<PracticeSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let mut session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    session.reset(&catalog).map_err(command_error)
}

#[tauri::command]
fn advance_practice(state: State<'_, AppState>) -> Result<PracticeSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let mut session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    let current = session.advance(&catalog).map_err(command_error)?;
    drop(session);
    drop(catalog);
    save_content(&state)?;
    Ok(current)
}

#[tauri::command]
fn select_word_chapter(
    state: State<'_, AppState>,
    chapter: usize,
) -> Result<PracticeSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let mut session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    let current = session
        .select_chapter(&catalog, chapter)
        .map_err(command_error)?;
    drop(session);
    drop(catalog);
    save_content(&state)?;
    Ok(current)
}

#[tauri::command]
fn get_content_catalog(state: State<'_, AppState>) -> Result<ContentCatalogSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    Ok(catalog.snapshot())
}

#[tauri::command]
fn select_practice_mode(
    state: State<'_, AppState>,
    mode: PracticeMode,
) -> Result<PracticeSnapshot, String> {
    let mut catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    catalog.set_mode(mode);
    drop(catalog);
    let current = snapshot(&state, 0)?;
    save_content(&state)?;
    Ok(current)
}

#[tauri::command]
fn select_article(state: State<'_, AppState>, id: String) -> Result<PracticeSnapshot, String> {
    let mut catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    catalog.select_article(&id).map_err(command_error)?;
    drop(catalog);
    let current = snapshot(&state, 0)?;
    save_content(&state)?;
    Ok(current)
}

#[tauri::command]
fn select_word_book(state: State<'_, AppState>, id: String) -> Result<PracticeSnapshot, String> {
    let mut catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    catalog.select_word_book(&id).map_err(command_error)?;
    drop(catalog);
    let current = snapshot(&state, 0)?;
    save_content(&state)?;
    Ok(current)
}

fn file_title(path: &Path) -> Result<String, String> {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "无法从文件名得到标题".to_owned())
}

fn article_from_path(path: &Path) -> Result<typelet_typing::Article, String> {
    let title = file_title(path)?;
    let bytes = fs::read(path).map_err(|error| format!("读取文章失败：{error}"))?;
    let text =
        String::from_utf8(bytes).map_err(|error| format!("文章不是有效的 UTF-8：{error}"))?;
    parse_article(&title, &path.to_string_lossy(), &text).map_err(command_error)
}

fn word_book_from_path(path: &Path) -> Result<(typelet_typing::WordBook, ImportReport), String> {
    let title = file_title(path)?;
    let bytes = fs::read(path).map_err(|error| format!("读取词表失败：{error}"))?;
    let text =
        String::from_utf8(bytes).map_err(|error| format!("词表不是有效的 UTF-8：{error}"))?;
    parse_word_book(&title, &path.to_string_lossy(), &text).map_err(command_error)
}

fn article_report(article: &typelet_typing::Article) -> ImportReport {
    ImportReport {
        title: article.title.clone(),
        count: article.sentences.len(),
        duplicate_count: 0,
        first_item: article.sentences.first().cloned(),
    }
}

#[tauri::command]
fn preview_article(path: String) -> Result<ImportReport, String> {
    let article = article_from_path(Path::new(&path))?;
    Ok(article_report(&article))
}

#[tauri::command]
fn preview_word_book(path: String) -> Result<ImportReport, String> {
    let (_, report) = word_book_from_path(Path::new(&path))?;
    Ok(report)
}

#[tauri::command]
fn import_article(state: State<'_, AppState>, path: String) -> Result<ImportResult, String> {
    let file_path = PathBuf::from(path);
    let article = article_from_path(&file_path)?;
    let report = article_report(&article);
    let mut catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    catalog.add_article(article);
    let catalog_snapshot = catalog.snapshot();
    drop(catalog);
    let current = snapshot(&state, 0)?;
    save_content(&state)?;
    Ok(ImportResult {
        report,
        catalog: catalog_snapshot,
        snapshot: current,
    })
}

#[tauri::command]
fn import_word_book(state: State<'_, AppState>, path: String) -> Result<ImportResult, String> {
    let file_path = PathBuf::from(path);
    let (book, report) = word_book_from_path(&file_path)?;
    let mut catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    catalog.add_word_book(book);
    let catalog_snapshot = catalog.snapshot();
    drop(catalog);
    let current = snapshot(&state, 0)?;
    save_content(&state)?;
    Ok(ImportResult {
        report,
        catalog: catalog_snapshot,
        snapshot: current,
    })
}

fn remove_collection(state: &AppState, sentence: bool, id: &str) -> Result<CatalogUpdate, String> {
    let mut catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    if sentence {
        catalog.remove_article(id).map_err(command_error)?;
    } else {
        catalog.remove_word_book(id).map_err(command_error)?;
    }
    let catalog_snapshot = catalog.snapshot();
    drop(catalog);
    let current = snapshot(state, 0)?;
    save_content(state)?;
    Ok(CatalogUpdate {
        catalog: catalog_snapshot,
        snapshot: current,
    })
}

#[tauri::command]
fn remove_article(state: State<'_, AppState>, id: String) -> Result<CatalogUpdate, String> {
    remove_collection(&state, true, &id)
}

#[tauri::command]
fn remove_word_book(state: State<'_, AppState>, id: String) -> Result<CatalogUpdate, String> {
    remove_collection(&state, false, &id)
}

fn use_x11_when_gnome_ignores_pin() {
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("GDK_BACKEND").is_some() {
            return;
        }
        // GNOME Wayland drops gtk_window_set_keep_above, so the pin button
        // lights up while other windows still cover Typelet. Mutter does honor
        // _NET_WM_STATE_ABOVE for XWayland clients.
        let on_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        let xwayland_available = std::env::var_os("DISPLAY").is_some();
        if !on_wayland || !xwayland_available {
            return;
        }
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let gnome = desktop
            .split([':', ';'])
            .any(|part| part.eq_ignore_ascii_case("GNOME"));
        if gnome {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }
}

fn main() {
    use_x11_when_gnome_ignores_pin();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
            let storage_path = data_dir.join("content.json");
            let stored = if storage_path.is_file() {
                let bytes = fs::read(&storage_path).map_err(|error| error.to_string())?;
                serde_json::from_slice::<StoredContentState>(&bytes)
                    .map_err(|error| error.to_string())?
            } else {
                StoredContentState::default()
            };
            let sentence_index = stored.sentence_index;
            let word_index = stored.word_index;
            let content = ContentCatalog::from_stored(stored).map_err(command_error)?;
            let session = PracticeSession::with_stored_progress(&content, sentence_index, word_index)
                .map_err(command_error)?;
            app.manage(AppState {
                content: Mutex::new(content),
                session: Mutex::new(session),
                storage_path,
            });
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_practice_snapshot,
            typing_snapshot,
            reset_practice,
            advance_practice,
            select_word_chapter,
            get_content_catalog,
            select_practice_mode,
            select_article,
            select_word_book,
            preview_article,
            preview_word_book,
            import_article,
            import_word_book,
            remove_article,
            remove_word_book,
            fonts::list_system_fonts,
            agents::get_agents_config,
            agents::set_agent_enabled,
            agents::get_agent_detail,
            agents::save_agent_detail
        ])
        .run(tauri::generate_context!())
        .expect("error while running Typelet");
}
