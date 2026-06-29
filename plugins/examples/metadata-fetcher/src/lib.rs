//! Manga Metadata Fetcher Plugin — chapter metadata demo.
//!
//! Demonstrates the `on_chapter_start` and `on_panel_detected` hooks.
//! When a chapter opens, this plugin mocks fetching external metadata
//! (synopsis, release date, rating) and appends classification tags
//! to detected panels.
//!
//! Build:
//! ```bash
//! cargo build --target wasm32-wasi --release
//! ```

use manga_reader_plugin::{
    register_plugin, ChapterInfo, HookAction, HookResult, PageInfo, PanelInfo, Plugin,
    PluginMetadata,
};

// ---------------------------------------------------------------------------
// Plugin struct & metadata
// ---------------------------------------------------------------------------

/// A demo metadata fetcher that enriches chapters and panel data.
///
/// In a real plugin this would scrape AniList, MAL, or a custom API
/// over WASI HTTP.
#[derive(Default)]
pub struct MetadataFetcherPlugin;

impl MetadataFetcherPlugin {
    /// Static metadata exposed to the host.
    pub const METADATA: PluginMetadata = PluginMetadata {
        id: "com.mangareader.metadata-fetcher",
        name: "Metadata Fetcher",
        version: "0.1.0",
        description: "Fetches chapter metadata and classifies detected panels",
        author: "Manga Reader Team",
    };

    /// Mock metadata lookup — returns fake data keyed by series + chapter.
    /// In production this would be an HTTP fetch to an API.
    fn fetch_metadata(chapter: &ChapterInfo) -> serde_json::Value {
        let series = chapter
            .series_id
            .as_deref()
            .unwrap_or("unknown-series");
        let ch = chapter.chapter_number;

        // Simulate different metadata per series
        match series {
            "one-piece" => serde_json::json!({
                "series": "One Piece",
                "chapter": ch,
                "title": chapter.title.as_deref().unwrap_or("Unknown"),
                "synopsis": "Luffy and the Straw Hat Pirates continue their adventure!",
                "release_date": "2026-06-15",
                "rating": "9.2/10",
                "page_count": chapter.total_pages.unwrap_or(0),
            }),
            "jujutsu-kaisen" => serde_json::json!({
                "series": "Jujutsu Kaisen",
                "chapter": ch,
                "title": chapter.title.as_deref().unwrap_or("Unknown"),
                "synopsis": "The sorcerers face new curses in the modern world.",
                "release_date": "2026-06-10",
                "rating": "8.9/10",
                "page_count": chapter.total_pages.unwrap_or(0),
            }),
            _ => serde_json::json!({
                "series": series,
                "chapter": ch,
                "title": chapter.title.as_deref().unwrap_or("Unknown"),
                "synopsis": "No synopsis available.",
                "release_date": null,
                "rating": "N/A",
                "page_count": chapter.total_pages.unwrap_or(0),
            }),
        }
    }

    /// Classify a panel based on its properties.
    fn classify_panel(panel: &PanelInfo) -> String {
        // Heuristic classification
        if panel.confidence < 0.3 {
            return "low-confidence".into();
        }
        if !panel.has_text {
            return "artwork".into();
        }
        // Large panels are likely splash pages or establishing shots
        if panel.bbox.width * panel.bbox.height > 0.4 {
            return "splash".into();
        }
        // Tall, narrow panels are often narrative boxes
        if panel.bbox.height > panel.bbox.width * 2.0 {
            return "narrative".into();
        }
        // Small panels are usually dialogue / reaction shots
        if panel.bbox.width * panel.bbox.height < 0.05 {
            return "reaction".into();
        }
        "dialogue".into()
    }
}

// ---------------------------------------------------------------------------
// Plugin trait implementation
// ---------------------------------------------------------------------------

impl Plugin for MetadataFetcherPlugin {
    /// Called when a new page opens — not used by this plugin.
    fn on_page_open(&self, _page: &PageInfo) -> Option<HookResult> {
        None
    }

    /// Called when a chapter starts.
    ///
    /// We "fetch" metadata for the chapter and return it as an override
    /// payload.  The host can display a chapter info card (synopsis,
    /// rating, release date) before the first page.
    fn on_chapter_start(&self, chapter: &ChapterInfo) -> Option<HookResult> {
        let metadata = Self::fetch_metadata(chapter);

        let message = format!(
            "Fetched metadata for {} ch.{}{}",
            chapter.series_id.as_deref().unwrap_or("unknown"),
            chapter.chapter_number,
            chapter
                .title
                .as_ref()
                .map(|t| format!(" — {}", t))
                .unwrap_or_default(),
        );

        Some(HookResult {
            action: HookAction::Override(
                serde_json::to_vec(&metadata).unwrap_or_default(),
            ),
            message: Some(message),
            data: Some(metadata),
        })
    }

    /// Called after panels have been detected on a page.
    ///
    /// We classify each panel and attach tags.  The host can use these
    /// tags to filter or style panels differently (e.g., colour-code
    /// dialogue vs. narrative boxes).
    fn on_panel_detected(&self, panels: &[PanelInfo]) -> Option<HookResult> {
        if panels.is_empty() {
            return None;
        }

        let classified: Vec<serde_json::Value> = panels
            .iter()
            .map(|panel| {
                let classification = Self::classify_panel(panel);
                serde_json::json!({
                    "bbox": {
                        "x": panel.bbox.x,
                        "y": panel.bbox.y,
                        "width": panel.bbox.width,
                        "height": panel.bbox.height,
                    },
                    "detected_text": panel.detected_text,
                    "confidence": panel.confidence,
                    "has_text": panel.has_text,
                    "tags": panel.tags,
                    "classification": classification,
                })
            })
            .collect();

        let data = serde_json::json!({
            "type": "panel_classification",
            "panels": classified,
            "summary": {
                "total": classified.len(),
                "dialogue": classified.iter().filter(|p| p["classification"] == "dialogue").count(),
                "artwork": classified.iter().filter(|p| p["classification"] == "artwork").count(),
                "splash": classified.iter().filter(|p| p["classification"] == "splash").count(),
                "narrative": classified.iter().filter(|p| p["classification"] == "narrative").count(),
                "reaction": classified.iter().filter(|p| p["classification"] == "reaction").count(),
            }
        });

        Some(HookResult {
            action: HookAction::Override(
                serde_json::to_vec(&data).unwrap_or_default(),
            ),
            message: Some(format!(
                "Classified {} panels ({} dialogue, {} artwork, {} splash, {} narrative, {} reaction)",
                classified.len(),
                data["summary"]["dialogue"].as_u64().unwrap_or(0),
                data["summary"]["artwork"].as_u64().unwrap_or(0),
                data["summary"]["splash"].as_u64().unwrap_or(0),
                data["summary"]["narrative"].as_u64().unwrap_or(0),
                data["summary"]["reaction"].as_u64().unwrap_or(0),
            )),
            data: Some(data),
        })
    }

    /// Called when text is selected — not used by this metadata plugin.
    fn on_text_selected(&self, _text: &str) -> Option<HookResult> {
        None
    }
}

// ---------------------------------------------------------------------------
// WASM entry points
// ---------------------------------------------------------------------------

register_plugin!(MetadataFetcherPlugin);
