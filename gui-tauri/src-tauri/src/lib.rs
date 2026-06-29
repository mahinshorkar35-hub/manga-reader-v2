use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

// ---------------------------------------------------------------
// Data models
// ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MangaSeries {
    pub id: String,
    pub title: String,
    pub author: String,
    pub cover_path: String,
    pub total_chapters: u32,
    pub genre: Vec<String>,
    pub status: String, // "Reading", "Completed", "Plan to Read", "Dropped"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub series_id: String,
    pub number: f32,
    pub title: String,
    pub page_count: u32,
    pub pages: Vec<PageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub index: u32,
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String, // "dark" or "light"
    pub reading_direction: String, // "ltr" or "rtl"
    pub page_fit_mode: String, // "fit-width", "fit-height", "original"
    pub show_page_numbers: bool,
    pub double_page_mode: bool,
    pub background_color: String,
    pub manga_root_path: String,
    pub tts_enabled: bool,
    pub tts_voice: String,
    pub tts_speed: f64,
    pub vision_backend: String,
    pub gemini_api_key: String,
    pub openai_api_key: String,
    pub elevenlabs_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub series_id: String,
    pub title: String,
    pub author: String,
    pub match_type: String, // "title", "author", "genre"
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_series: u32,
    pub total_chapters: u32,
    pub total_pages: u32,
    pub reading_count: u32,
    pub completed_count: u32,
    pub plan_to_read_count: u32,
}

// ---------------------------------------------------------------
// Application state
// ---------------------------------------------------------------

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub library: Mutex<Vec<MangaSeries>>,
    pub chapters_cache: Mutex<HashMap<String, Vec<Chapter>>>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            settings: Mutex::new(AppSettings::default()),
            library: Mutex::new(Vec::new()),
            chapters_cache: Mutex::new(HashMap::new()),
            data_dir,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            reading_direction: "ltr".to_string(),
            page_fit_mode: "fit-width".to_string(),
            show_page_numbers: true,
            double_page_mode: false,
            background_color: "#1a1a2e".to_string(),
            manga_root_path: String::new(),
            tts_enabled: false,
            tts_voice: "af_heart".to_string(),
            tts_speed: 1.0,
            vision_backend: "gemini".to_string(),
            gemini_api_key: String::new(),
            openai_api_key: String::new(),
            elevenlabs_api_key: String::new(),
        }
    }
}

// ---------------------------------------------------------------
// IPC Commands — Library
// ---------------------------------------------------------------

#[tauri::command]
fn get_library(state: State<AppState>) -> Result<Vec<MangaSeries>, String> {
    let library = state.library.lock().map_err(|e| e.to_string())?;
    Ok(library.clone())
}

#[tauri::command]
fn get_library_stats(state: State<AppState>) -> Result<LibraryStats, String> {
    let library = state.library.lock().map_err(|e| e.to_string())?;
    let total_series = library.len() as u32;
    let reading_count = library.iter().filter(|s| s.status == "Reading").count() as u32;
    let completed_count = library.iter().filter(|s| s.status == "Completed").count() as u32;
    let plan_to_read_count = library.iter().filter(|s| s.status == "Plan to Read").count() as u32;

    let total_chapters: u32 = library.iter().map(|s| s.total_chapters).sum();

    Ok(LibraryStats {
        total_series,
        total_chapters,
        total_pages: 0,
        reading_count,
        completed_count,
        plan_to_read_count,
    })
}

#[tauri::command]
fn get_series(series_id: String, state: State<AppState>) -> Result<Option<MangaSeries>, String> {
    let library = state.library.lock().map_err(|e| e.to_string())?;
    Ok(library.iter().find(|s| s.id == series_id).cloned())
}

#[tauri::command]
fn update_series_status(
    series_id: String,
    status: String,
    state: State<AppState>,
) -> Result<(), String> {
    let mut library = state.library.lock().map_err(|e| e.to_string())?;
    if let Some(series) = library.iter_mut().find(|s| s.id == series_id) {
        series.status = status;
        Ok(())
    } else {
        Err(format!("Series '{}' not found", series_id))
    }
}

#[tauri::command]
fn delete_series(series_id: String, state: State<AppState>) -> Result<(), String> {
    let mut library = state.library.lock().map_err(|e| e.to_string())?;
    library.retain(|s| s.id != series_id);
    Ok(())
}

// ---------------------------------------------------------------
// IPC Commands — Chapters & Reading
// ---------------------------------------------------------------

#[tauri::command]
fn get_chapters(series_id: String, state: State<AppState>) -> Result<Vec<Chapter>, String> {
    let cache = state.chapters_cache.lock().map_err(|e| e.to_string())?;
    Ok(cache.get(&series_id).cloned().unwrap_or_default())
}

#[tauri::command]
fn get_chapter(chapter_id: String, state: State<AppState>) -> Result<Option<Chapter>, String> {
    let cache = state.chapters_cache.lock().map_err(|e| e.to_string())?;
    for chapters in cache.values() {
        if let Some(ch) = chapters.iter().find(|c| c.id == chapter_id) {
            return Ok(Some(ch.clone()));
        }
    }
    Ok(None)
}

#[tauri::command]
fn get_page_image(
    series_id: String,
    chapter_id: String,
    page_index: u32,
    state: State<AppState>,
) -> Result<String, String> {
    let cache = state.chapters_cache.lock().map_err(|e| e.to_string())?;
    let chapters = cache
        .get(&series_id)
        .ok_or_else(|| format!("Series '{}' not found in cache", series_id))?;
    let chapter = chapters
        .iter()
        .find(|c| c.id == chapter_id)
        .ok_or_else(|| format!("Chapter '{}' not found", chapter_id))?;
    let page = chapter
        .pages
        .iter()
        .find(|p| p.index == page_index)
        .ok_or_else(|| format!("Page {} not found in chapter", page_index))?;

    // Read the image file and encode as base64
    let img_path = &page.path;
    let img_data = std::fs::read(img_path)
        .map_err(|e| format!("Failed to read image '{}': {}", img_path, e))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
    // Determine MIME type from extension
    let ext = std::path::Path::new(img_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    };
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
fn scan_manga_directory(path: String, state: State<AppState>) -> Result<Vec<MangaSeries>, String> {
    let scan_path = PathBuf::from(&path);
    if !scan_path.exists() {
        return Err(format!("Directory '{}' does not exist", path));
    }
    if !scan_path.is_dir() {
        return Err(format!("'{}' is not a directory", path));
    }

    let mut series_list = Vec::new();
    let entries = std::fs::read_dir(&scan_path)
        .map_err(|e| format!("Failed to read directory '{}': {}", path, e))?;

    for entry in entries.flatten() {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Try to find a cover image
        let cover = find_cover_image(&dir_path);

        // Count subdirectories as chapters
        let chapter_count = std::fs::read_dir(&dir_path)
            .map(|entries| entries.filter(|e| e.as_ref().map(|e| e.path().is_dir()).unwrap_or(false)).count() as u32)
            .unwrap_or(0);

        let series = MangaSeries {
            id: uuid::Uuid::new_v4().to_string(),
            title: dir_name,
            author: "Unknown".to_string(),
            cover_path: cover.unwrap_or_default(),
            total_chapters: chapter_count,
            genre: Vec::new(),
            status: "Plan to Read".to_string(),
        };
        series_list.push(series);
    }

    // Update the library state
    let mut library = state.library.lock().map_err(|e| e.to_string())?;
    library.extend(series_list.clone());

    Ok(series_list)
}

fn find_cover_image(dir: &std::path::Path) -> Option<String> {
    let cover_patterns = [
        "cover.jpg", "cover.png", "cover.webp",
        "Cover.jpg", "Cover.png", "Cover.webp",
        "folder.jpg", "folder.png",
        "Folder.jpg", "Folder.png",
        "thumbnail.jpg", "thumbnail.png",
        "Thumbnail.jpg", "Thumbnail.png",
    ];
    for pattern in &cover_patterns {
        let candidate = dir.join(pattern);
        if candidate.exists() {
            return candidate.to_str().map(|s| s.to_string());
        }
    }
    // Fallback: first image in directory
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "webp") {
                        return path.to_str().map(|s| s.to_string());
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------
// IPC Commands — Settings
// ---------------------------------------------------------------

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
fn save_settings(new_settings: AppSettings, state: State<AppState>) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    *settings = new_settings.clone();

    // Persist to disk
    let settings_path = state.data_dir.join("settings.json");
    let json = serde_json::to_string_pretty(&new_settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&settings_path, &json)
        .map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}

#[tauri::command]
fn reset_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let default = AppSettings::default();
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    *settings = default.clone();

    let settings_path = state.data_dir.join("settings.json");
    let json = serde_json::to_string_pretty(&default)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    let _ = std::fs::write(&settings_path, &json);

    Ok(default)
}

// ---------------------------------------------------------------
// IPC Commands — Search
// ---------------------------------------------------------------

#[tauri::command]
fn search_series(query: String, state: State<AppState>) -> Result<Vec<SearchResult>, String> {
    let library = state.library.lock().map_err(|e| e.to_string())?;
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for series in library.iter() {
        let title_lower = series.title.to_lowercase();
        let author_lower = series.author.to_lowercase();

        // Title match (highest score)
        if title_lower.contains(&query_lower) {
            let score = if title_lower.starts_with(&query_lower) {
                1.0
            } else {
                0.8
            };
            results.push(SearchResult {
                series_id: series.id.clone(),
                title: series.title.clone(),
                author: series.author.clone(),
                match_type: "title".to_string(),
                score,
            });
            continue;
        }

        // Author match
        if author_lower.contains(&query_lower) {
            results.push(SearchResult {
                series_id: series.id.clone(),
                title: series.title.clone(),
                author: series.author.clone(),
                match_type: "author".to_string(),
                score: 0.6,
            });
            continue;
        }

        // Genre match
        for genre in &series.genre {
            if genre.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    series_id: series.id.clone(),
                    title: series.title.clone(),
                    author: series.author.clone(),
                    match_type: "genre".to_string(),
                    score: 0.4,
                });
                break;
            }
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(results)
}

// ---------------------------------------------------------------
// IPC Commands — File System helpers
// ---------------------------------------------------------------

#[tauri::command]
fn get_manga_root(state: State<AppState>) -> Result<String, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.manga_root_path.clone())
}

#[tauri::command]
fn set_manga_root(path: String, state: State<AppState>) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.manga_root_path = path;
    Ok(())
}

#[tauri::command]
fn list_directories(path: String) -> Result<Vec<String>, String> {
    let dir_path = PathBuf::from(&path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Invalid directory: '{}'", path));
    }

    let mut dirs = Vec::new();
    let entries = std::fs::read_dir(&dir_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                dirs.push(name.to_string());
            }
        }
    }

    dirs.sort();
    Ok(dirs)
}

#[tauri::command]
fn file_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
}

// ---------------------------------------------------------------
// App entry
// ---------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Determine data directory
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("manga-reader-v2");

            // Ensure data directory exists
            std::fs::create_dir_all(&data_dir)
                .expect("Failed to create app data directory");

            // Load saved settings if they exist
            let settings_path = data_dir.join("settings.json");
            let app_state = AppState::new(data_dir.clone());

            if settings_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&settings_path) {
                    if let Ok(saved) = serde_json::from_str::<AppSettings>(&content) {
                        let mut settings = app_state.settings.lock().unwrap();
                        *settings = saved;
                    }
                }
            }

            // Load saved library if it exists
            let library_path = data_dir.join("library.json");
            if library_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&library_path) {
                    if let Ok(saved) = serde_json::from_str::<Vec<MangaSeries>>(&content) {
                        let mut library = app_state.library.lock().unwrap();
                        *library = saved;
                    }
                }
            }

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Library
            get_library,
            get_library_stats,
            get_series,
            update_series_status,
            delete_series,
            // Chapters & Reading
            get_chapters,
            get_chapter,
            get_page_image,
            scan_manga_directory,
            // Settings
            get_settings,
            save_settings,
            reset_settings,
            // Search
            search_series,
            // File system helpers
            get_manga_root,
            set_manga_root,
            list_directories,
            file_exists,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Manga Reader v2");
}
