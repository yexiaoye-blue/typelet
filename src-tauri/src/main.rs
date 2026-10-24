#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Manager, State};
use typelet_typing::{
    parse_article, parse_word_book, ContentCatalog, ContentCatalogSnapshot, ContentError,
    ImportReport, PracticeMode, PracticeOrder, PracticeSession, PracticeSnapshot,
    StoredContentState,
};

mod agents;
mod fonts;
mod key_sound;

use key_sound::{load_key_sound, play_key_sound};

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

#[tauri::command]
fn log_key_sound(app: tauri::AppHandle, line: String) -> Result<(), String> {
    key_sound::log_frontend(&app, line);
    Ok(())
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
    stored.positions = session.positions().clone();
    let json = serde_json::to_vec_pretty(&stored).map_err(|error| error.to_string())?;
    // Write to a sibling file and rename over the target so a crash mid-save
    // cannot leave a half-written content.json behind.
    let temp_path = state.storage_path.with_extension("json.tmp");
    fs::write(&temp_path, json).map_err(|error| format!("{}: {error}", temp_path.display()))?;
    fs::rename(&temp_path, &state.storage_path)
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

fn content_snapshot(state: &AppState) -> Result<ContentCatalogSnapshot, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    Ok(catalog.snapshot_with_positions(session.positions(), session.chapter_length()))
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
fn select_practice_item(
    state: State<'_, AppState>,
    item_index: usize,
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
        .select_item(&catalog, item_index)
        .map_err(command_error)?;
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
    content_snapshot(&state)
}

#[tauri::command]
fn set_practice_layout(
    state: State<'_, AppState>,
    chapter_length: usize,
    order: PracticeOrder,
) -> Result<CatalogUpdate, String> {
    let catalog = state
        .content
        .lock()
        .map_err(|_| "内容状态锁定失败".to_owned())?;
    let mut session = state
        .session
        .lock()
        .map_err(|_| "练习状态锁定失败".to_owned())?;
    let snapshot = session
        .set_layout(&catalog, chapter_length, order)
        .map_err(command_error)?;
    let listed = catalog.snapshot_with_positions(session.positions(), session.chapter_length());
    Ok(CatalogUpdate {
        catalog: listed,
        snapshot,
    })
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
    drop(catalog);
    let current = snapshot(&state, 0)?;
    let catalog_snapshot = content_snapshot(&state)?;
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
    drop(catalog);
    let current = snapshot(&state, 0)?;
    let catalog_snapshot = content_snapshot(&state)?;
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
    drop(catalog);
    let current = snapshot(state, 0)?;
    let catalog_snapshot = content_snapshot(state)?;
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

// A corrupt or unreadable content.json must not keep the app from starting:
// park the broken file next to the real one and fall back to built-in content.
fn load_stored_state(data_dir: &Path, storage_path: &Path) -> StoredContentState {
    if !storage_path.is_file() {
        return StoredContentState::default();
    }
    let parsed = fs::read(storage_path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<StoredContentState>(&bytes).map_err(|error| error.to_string())
        });
    match parsed {
        Ok(stored) => stored,
        Err(error) => {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            let mut backup = data_dir.join(format!("content.json.corrupt-{stamp}"));
            let mut suffix = 0u32;
            while backup.exists() {
                suffix += 1;
                backup = data_dir.join(format!("content.json.corrupt-{stamp}-{suffix}"));
            }
            if let Err(rename_error) = fs::rename(storage_path, &backup) {
                eprintln!(
                    "content.json 无法解析（{error}），且备份到 {} 失败：{rename_error}",
                    backup.display()
                );
            } else {
                eprintln!(
                    "content.json 无法解析（{error}），已改名为 {}，使用内置内容继续",
                    backup.display()
                );
            }
            StoredContentState::default()
        }
    }
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
            let stored = load_stored_state(&data_dir, &storage_path);
            let sentence_index = stored.sentence_index;
            let word_index = stored.word_index;
            let mut positions = stored.positions.clone();
            if let Some(id) = stored.article_id.clone() {
                positions.insert(id, sentence_index);
            }
            if let Some(id) = stored.word_book_id.clone() {
                positions.insert(id, word_index);
            }
            let content = ContentCatalog::from_stored(stored).map_err(command_error)?;
            let session =
                PracticeSession::with_positions(&content, sentence_index, word_index, positions)
                    .map_err(command_error)?;
            app.manage(AppState {
                content: Mutex::new(content),
                session: Mutex::new(session),
                storage_path,
            });
            app.manage(key_sound::KeyAudio::new());
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
            select_practice_item,
            select_word_chapter,
            get_content_catalog,
            set_practice_layout,
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
            agents::save_agent_detail,
            log_key_sound,
            load_key_sound,
            play_key_sound
        ])
        .run(tauri::generate_context!())
        .expect("error while running Typelet");
}
