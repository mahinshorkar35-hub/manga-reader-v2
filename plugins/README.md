# Manga Reader WASM Plugin System

This directory defines the **WebAssembly plugin interface** for Manga Reader v2.
Plugins are compiled to the `wasm32-wasi` target and loaded at runtime to
extend or modify reader behaviour without touching the core application code.

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Plugin Trait](#plugin-trait)
- [Data Types](#data-types)
- [How to Write a Plugin](#how-to-write-a-plugin)
- [Build Instructions](#build-instructions)
- [Plugin Registration & Lifecycle](#plugin-registration--lifecycle)
- [Example Plugins](#example-plugins)
- [Best Practices](#best-practices)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                  Manga Reader Host                    │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────┐  │
│  │ Page     │  │ Chapter  │  │ Panel Detector    │  │
│  │ Renderer │  │ Manager  │  │ Engine            │  │
│  └────┬─────┘  └────┬─────┘  └────────┬──────────┘  │
│       │              │                 │              │
│  ┌────▼──────────────▼─────────────────▼──────────┐  │
│  │              WASM Runtime (wasmtime)            │  │
│  │  ┌─────────────┐  ┌──────────────┐             │  │
│  │  │ Translator  │  │ Metadata     │  ...more     │  │
│  │  │ Plugin      │  │ Fetcher      │  plugins     │  │
│  │  └─────────────┘  └──────────────┘             │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

Each plugin is a **separate `.wasm` file** loaded into a sandboxed WASM
runtime. The host calls hook methods on the plugin when events occur; the
plugin returns an optional `HookResult` to influence host behaviour.

---

## Plugin Trait

Every WASM plugin must implement the `Plugin` trait defined in
[`traits/plugin.rs`](traits/plugin.rs):

```rust
pub trait Plugin {
    fn on_page_open(page: &PageInfo) -> Option<HookResult>;
    fn on_chapter_start(chapter: &ChapterInfo) -> Option<HookResult>;
    fn on_panel_detected(panels: &[PanelInfo]) -> Option<HookResult>;
    fn on_text_selected(text: &str) -> Option<HookResult>;
}
```

All methods are **hooks**. Return `None` to indicate "no reaction" — the
host treats this identically to `Some(HookResult::default())`.

### Hook reference

| Method                  | When it's called                                   | Typical use cases                               |
|-------------------------|----------------------------------------------------|-------------------------------------------------|
| `on_page_open`          | A new page image is displayed in the reader.       | Image filters, overlays, upscaling.             |
| `on_chapter_start`      | The user opens a chapter (first page).             | Prefetch translations, fetch metadata.          |
| `on_panel_detected`     | Panel / text-bubble detection finishes for a page. | Re-classify panels, attach tags, merge regions. |
| `on_text_selected`      | The user highlights / selects text in a panel.     | Dictionary lookup, inline translation, vocab.   |

---

## Data Types

### `PageInfo`

```rust
pub struct PageInfo {
    pub page_index: u32,
    pub image_path: String,
    pub width: u32,
    pub height: u32,
    pub metadata: HashMap<String, String>,
}
```

### `ChapterInfo`

```rust
pub struct ChapterInfo {
    pub chapter_number: f64,
    pub volume_number: Option<f64>,
    pub title: Option<String>,
    pub series_id: Option<String>,
    pub total_pages: Option<u32>,
}
```

### `PanelInfo`

```rust
pub struct PanelInfo {
    pub bbox: BoundingBox,
    pub detected_text: Option<String>,
    pub confidence: f64,
    pub has_text: bool,
    pub tags: Vec<String>,
}
```

### `BoundingBox`

```rust
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,    // all normalised 0.0–1.0
}
```

### `HookResult`

```rust
pub struct HookResult {
    pub action: HookAction,   // Continue | Override(Vec<u8>) | Block
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}
```

- **`Continue`** — let the host proceed as normal.
- **`Override(data)`** — replace the host's output with `data`.
- **`Block`** — prevent the default action entirely.

---

## How to Write a Plugin

### 1. Create the crate

```
examples/my-plugin/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 2. Add dependencies

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
manga-reader-plugin = { path = "../../traits" }   # or publish as a crate

[profile.release]
opt-level = "s"       # optimise for size
lto = true
strip = true
```

### 3. Implement the trait

```rust
use manga_reader_plugin::{Plugin, PageInfo, ChapterInfo, PanelInfo, HookResult};
use manga_reader_plugin::register_plugin;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn on_page_open(page: &PageInfo) -> Option<HookResult> {
        // Your code here …
        None
    }

    fn on_chapter_start(chapter: &ChapterInfo) -> Option<HookResult> { None }
    fn on_panel_detected(panels: &[PanelInfo]) -> Option<HookResult> { None }
    fn on_text_selected(text: &str) -> Option<HookResult> { None }
}

register_plugin!(MyPlugin);
```

### 4. Build

```bash
rustup target add wasm32-wasi
cargo build --target wasm32-wasi --release
```

The output `.wasm` file lives at:
`target/wasm32-wasi/release/my-plugin.wasm`

### 5. Deploy

Copy the `.wasm` file into the reader's plugins directory (e.g.
`~/.config/manga-reader/plugins/`). The host discovers and loads plugins on
startup.

---

## Plugin Registration & Lifecycle

The `register_plugin!` macro generates three C-ABI exports that the WASM
runtime uses to manage the plugin:

| Export              | Purpose                                              |
|---------------------|------------------------------------------------------|
| `_plugin_metadata`  | Returns a `PluginMetadata` struct (id, name, ver…).  |
| `_plugin_create`    | Returns a `Box<dyn Plugin>` as a raw pointer.        |
| `_plugin_destroy`   | Frees the plugin instance (takes the raw pointer).   |

The host calls these in order: **metadata → create → (hooks)* → destroy**.

---

## Example Plugins

Two complete, working examples are provided:

| Plugin              | Description                                        | Key hooks used          |
|---------------------|----------------------------------------------------|-------------------------|
| [`translator`](examples/translator/) | Inline text translation via a mock API    | `on_text_selected`, `on_page_open` |
| [`metadata-fetcher`](examples/metadata-fetcher/) | Fetches chapter metadata from a remote source | `on_chapter_start`, `on_panel_detected` |

To build both:

```bash
cd examples/translator
cargo build --target wasm32-wasi --release

cd ../metadata-fetcher
cargo build --target wasm32-wasi --release
```

---

## Best Practices

1. **Keep it small.** WASM modules are downloaded/loaded on demand; prefer
   `opt-level = "s"` and `lto = true` in release builds.

2. **Be non-blocking.** All hooks run on the UI thread. If you need to do
   I/O (HTTP, file access), do it synchronously but keep it fast, or
   coordinate with the host via `HookResult::data`.

3. **Use `None` for pass-through.** Don't return
   `Some(HookResult::default())` — returning `None` is slightly cheaper
   because the host short-circuits its deserialisation path.

4. **Version your metadata.** Bump the `version` field in `PluginMetadata`
   when your plugin's data format or behaviour changes so the host can
   manage compatibility.

5. **Test with `wasmtime` directly.** Before deploying, validate the module:

   ```bash
   wasmtime run --dir=. my-plugin.wasm
   ```

6. **Sandbox limitations.** WASM has no direct access to the filesystem,
   network, or OS (unless WASI preview-2 sockets are enabled). Use the hook
   interface to communicate with the host.

---

## License

The plugin trait definitions are part of Manga Reader v2.
Example plugins are provided under the MIT license.
