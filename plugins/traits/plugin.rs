//! Core plugin trait definitions for the Manga Reader WASM plugin system.
//!
//! This module defines the `Plugin` trait that all WASM plugins must implement,
//! along with the supporting data types used by the host-to-plugin interface.
//!
//! # Architecture
//!
//! The plugin system loads WebAssembly modules compiled to the `wasm32-wasi` target.
//! Each plugin exports a `_plugin_create()` function that returns a boxed `dyn Plugin`,
//! registered via the `register_plugin!()` macro.
//!
//! # Data Flow
//!
//! ```text
//! ┌──────────────┐     Hook Events      ┌──────────────┐
//! │  Manga       │ ──────────────────►   │   WASM       │
//! │  Reader      │                      │   Plugin      │
//! │  (Host)      │ ◄──────────────────  │  (Guest)      │
//! └──────────────┘   Option<HookResult>  └──────────────┘
//! ```

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data types – exchanged between host and plugin
// ---------------------------------------------------------------------------

/// Represents a single manga page the reader has opened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Zero-based index of the page within the current chapter.
    pub page_index: u32,
    /// Path to the rendered image file on disk (host-local).
    pub image_path: String,
    /// Width of the page image in pixels.
    pub width: u32,
    /// Height of the page image in pixels.
    pub height: u32,
    /// Optional metadata key-value pairs (e.g., detected language, OCR hints).
    pub metadata: std::collections::HashMap<String, String>,
}

/// Represents the start of a new chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    /// Chapter number (e.g., 1, 2, 3.5).
    pub chapter_number: f64,
    /// Volume number, if applicable.
    pub volume_number: Option<f64>,
    /// Chapter title, if available.
    pub title: Option<String>,
    /// Manga series identifier (e.g., "one-piece", "jujutsu-kaisen").
    pub series_id: Option<String>,
    /// Total number of pages in this chapter, if known.
    pub total_pages: Option<u32>,
}

/// Information about a detected speech bubble or panel region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelInfo {
    /// Bounding box of the panel – x, y, width, height (normalised 0.0–1.0).
    pub bbox: BoundingBox,
    /// Optional OCR-extracted text found inside this panel.
    pub detected_text: Option<String>,
    /// Confidence score of the panel detection (0.0 – 1.0).
    pub confidence: f64,
    /// Whether this panel contains recognised text.
    pub has_text: bool,
    /// Arbitrary tags the plugin can attach (e.g., `"dialogue"`, `"sfx"`, `"narrative"`).
    pub tags: Vec<String>,
}

/// Normalised rectangle coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The result a plugin returns after handling a hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookAction {
    /// Allow normal processing to continue (no-op).
    Continue,
    /// Replace or augment the default behaviour with custom data.
    Override(Vec<u8>),
    /// Block the default action entirely.
    Block,
}

/// Payload returned from a plugin hook invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// What the host should do after this hook.
    pub action: HookAction,
    /// Human-readable message for logging / debug overlays.
    pub message: Option<String>,
    /// Arbitrary JSON payload the host can forward to the renderer.
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Default implementations
// ---------------------------------------------------------------------------

impl Default for HookResult {
    fn default() -> Self {
        Self {
            action: HookAction::Continue,
            message: None,
            data: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin trait — every WASM plugin must implement this
// ---------------------------------------------------------------------------

/// The core trait that every WASM plugin must implement.
///
/// Each method is a **hook** — the host calls it when a relevant event occurs.
/// Return `None` to signal "no reaction" (equivalent to
/// `Some(HookResult { action: HookAction::Continue, .. })`).
pub trait Plugin {
    /// Called when the reader opens a new page.
    ///
    /// Use this hook to:
    /// - Apply image filters or overlays.
    /// - Trigger AI upscaling / colourisation.
    /// - Add watermark or annotation layers.
    fn on_page_open(&self, page: &PageInfo) -> Option<HookResult>;

    /// Called when a new chapter starts.
    ///
    /// Use this hook to:
    /// - Prefetch external translations.
    /// - Initialise chapter-level state.
    /// - Fetch metadata (synopsis, summaries).
    fn on_chapter_start(&self, chapter: &ChapterInfo) -> Option<HookResult>;

    /// Called after panels / speech bubbles have been detected on a page.
    ///
    /// Use this hook to:
    /// - Re-classify or tag panels (e.g., mark as "flashback").
    /// - Merge/split overlapping regions.
    /// - Attach confidence thresholds.
    fn on_panel_detected(&self, panels: &[PanelInfo]) -> Option<HookResult>;

    /// Called when the user selects/highlights text in a panel.
    ///
    /// Use this hook to:
    /// - Look up the selected text in a dictionary.
    /// - Provide inline translation.
    /// - Save to a personal vocabulary list.
    fn on_text_selected(&self, text: &str) -> Option<HookResult>;
}

// ---------------------------------------------------------------------------
// Plugin metadata – discovered via WASM exports
// ---------------------------------------------------------------------------

/// Static metadata every plugin must expose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique identifier (e.g., `"com.example.translator"`).
    pub id: &'static str,
    /// Human-readable name (e.g., `"DeepL Translator"`).
    pub name: &'static str,
    /// Semantic version.
    pub version: &'static str,
    /// Short description of what the plugin does.
    pub description: &'static str,
    /// Author / organisation.
    pub author: &'static str,
}

// ---------------------------------------------------------------------------
// Registration helper
// ---------------------------------------------------------------------------

/// Macro that generates the C-ABI entry points needed by the WASM runtime.
///
/// Call it once at the crate root with your plugin type:
///
/// ```ignore
/// register_plugin!(MyTranslatorPlugin);
/// ```
///
/// It exports:
/// - `_plugin_metadata()` → `PluginMetadata`
/// - `_plugin_create()`   → `Box<dyn Plugin>`
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn _plugin_metadata() -> $crate::PluginMetadata {
            <$plugin_type>::METADATA
        }

        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut dyn $crate::Plugin {
            let plugin = <$plugin_type>::default();
            let boxed: Box<dyn $crate::Plugin> = Box::new(plugin);
            Box::into_raw(boxed)
        }

        #[no_mangle]
        pub unsafe extern "C" fn _plugin_destroy(ptr: *mut dyn $crate::Plugin) {
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr);
            }
        }
    };
}
