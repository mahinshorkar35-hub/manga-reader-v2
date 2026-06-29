//! Manga Translator Plugin — inline translation demo.
//!
//! Demonstrates the `on_text_selected` and `on_page_open` hooks.
//! When the user selects text in a panel, this plugin mocks a translation
//! lookup and returns a replacement string that the host can display.
//!
//! Build:
//! ```bash
//! cargo build --target wasm32-wasi --release
//! ```

use manga_reader_plugin::{
    register_plugin, BoundingBox, ChapterInfo, HookAction, HookResult, PageInfo, PanelInfo, Plugin,
    PluginMetadata,
};

// ---------------------------------------------------------------------------
// Plugin struct & metadata
// ---------------------------------------------------------------------------

/// A demo translator that replaces English text with a "translated" variant.
///
/// In a real plugin this would call an external translation API (DeepL,
/// Google Translate, etc.) over WASI HTTP or via a host bridge.
pub struct TranslatorPlugin;

impl TranslatorPlugin {
    /// Static metadata exposed to the host.
    pub const METADATA: PluginMetadata = PluginMetadata {
        id: "com.mangareader.translator",
        name: "Inline Translator",
        version: "0.1.0",
        description: "Translates selected text using a mock translation engine",
        author: "Manga Reader Team",
    };

    /// Internal dictionary mapping English phrases to their "translations".
    /// In production this would be a call to an external API.
    fn mock_translate(text: &str) -> String {
        let dictionary: &[(&str, &str)] = &[
            ("Hello", "こんにちは"),
            ("Goodbye", "さようなら"),
            ("Thank you", "ありがとう"),
            ("Sorry", "ごめんなさい"),
            ("What", "何"),
            ("Why", "なぜ"),
            ("Who", "誰"),
            ("Where", "どこ"),
            ("When", "いつ"),
            ("How", "どうやって"),
            ("I", "私"),
            ("You", "あなた"),
            ("He", "彼"),
            ("She", "彼女"),
            ("We", "私たち"),
            ("They", "彼ら"),
            ("Yes", "はい"),
            ("No", "いいえ"),
            ("Good", "良い"),
            ("Bad", "悪い"),
            ("Big", "大きい"),
            ("Small", "小さい"),
        ];

        for (en, translated) in dictionary {
            if text.contains(en) {
                return text.replace(en, translated);
            }
        }

        // Fallback: prepend a translation marker
        format!("[TR: {}]", text)
    }

    /// Simulates adding a translation overlay to the page image.
    /// Returns a JSON payload the host renderer can use to show a banner.
    fn translation_overlay(page: &PageInfo) -> Option<HookResult> {
        let data = serde_json::json!({
            "type": "translation_overlay",
            "page_index": page.page_index,
            "message": format!("Translations available for page {}", page.page_index + 1),
            "style": {
                "position": "top-right",
                "background": "rgba(0, 0, 0, 0.7)",
                "color": "#ffffff",
                "font_size": 14,
            }
        });

        Some(HookResult {
            action: HookAction::Override(
                serde_json::to_vec(&data).unwrap_or_default(),
            ),
            message: Some("Translation overlay applied".into()),
            data: Some(data),
        })
    }
}

// ---------------------------------------------------------------------------
// Plugin trait implementation
// ---------------------------------------------------------------------------

impl Plugin for TranslatorPlugin {
    /// Called when a new page opens.
    ///
    /// Here we inject a transparent overlay that signals translations are
    /// available.  The host renderer can parse the returned JSON and display
    /// a floating "Translate" button.
    fn on_page_open(page: &PageInfo) -> Option<HookResult> {
        // Only show the overlay on pages that look like they contain dialogue
        // (heuristic: pages wider than 700 px are likely spreads with text).
        if page.width > 700 {
            Self::translation_overlay(page)
        } else {
            None
        }
    }

    /// Called when a chapter starts — not used by this translator plugin,
    /// but we could pre-fetch a translation glossary here.
    fn on_chapter_start(_chapter: &ChapterInfo) -> Option<HookResult> {
        None
    }

    /// Called after panel detection — not used by the translator.
    fn on_panel_detected(_panels: &[PanelInfo]) -> Option<HookResult> {
        None
    }

    /// Called when the user selects text.
    ///
    /// This is the primary hook for this plugin.  We "translate" the
    /// selected text using our mock dictionary and return the result so
    /// the host can show it in a tooltip or replace the selection.
    fn on_text_selected(text: &str) -> Option<HookResult> {
        // Ignore very short selections (accidental taps).
        let trimmed = text.trim();
        if trimmed.len() < 2 {
            return None;
        }

        let translated = Self::mock_translate(trimmed);

        let data = serde_json::json!({
            "original": trimmed,
            "translated": translated,
            "engine": "mock-dict-v1",
        });

        Some(HookResult {
            action: HookAction::Override(
                serde_json::to_vec(&data).unwrap_or_default(),
            ),
            message: Some(format!("Translated: {}", translated)),
            data: Some(data),
        })
    }
}

// ---------------------------------------------------------------------------
// WASM entry points
// ---------------------------------------------------------------------------

register_plugin!(TranslatorPlugin);
