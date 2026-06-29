# Manga Reader v2 — Multi-Language Edition

The ultimate manga/manhwa/comic reading application, rebuilt from the ground up with the best technology for every layer.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    GUI LAYER                         │
│  C# WinUI 3 (Windows native)  |  Tauri v2 (Cross-plat)│
└──────────────────────┬──────────────────────────────┘
                       │ IPC (JSON-RPC over TCP/WebSocket)
┌──────────────────────┴──────────────────────────────┐
│               RUST CORE ENGINE                       │
│  Archive IO · Image Pipeline · Panel Detection       │
│  SQLite DB · Tantivy Search · WASM Plugin Host      │
└──────┬──────────────────────────────────────────────┘
       │ IPC (JSON-RPC over TCP)
┌──────┴──────────────────────────────────────────────┐
│             PYTHON AI PIPELINE (sidecar)              │
│  Gemini Vision · Kokoro TTS · Manga OCR · LLM       │
└─────────────────────────────────────────────────────┘
```

## Stack per Layer

| Layer | Language | Technology |
|-------|----------|------------|
| **Core Engine** | Rust | Tokio, image-rs, rusqlite, tantivy, memmap2, wgpu, ort |
| **GUI (Windows)** | C# | WinUI 3, SkiaSharp, CommunityToolkit.Mvvm |
| **GUI (Cross-platform)** | Rust + JS | Tauri v2, Svelte 4, Canvas2D |
| **AI Pipeline** | Python | Kokoro TTS, Gemini API, manga-ocr, EasyOCR |
| **Plugin System** | Rust + WASM | wasm32-wasi, serde |

## Project Structure

```
manga-reader-v2/
├── engine/              # Rust core engine (114 KB, 17 files)
│   ├── src/
│   │   ├── archive/     # CBZ, CBR, CB7, folder extraction
│   │   ├── image/       # Decode, cache, resize pipeline
│   │   ├── panel/       # Gutter-based panel detection
│   │   ├── db/          # SQLite data access + migrations
│   │   ├── ipc/         # JSON-RPC 2.0 TCP server
│   │   └── search/      # Tantivy full-text search
│   └── Cargo.toml
├── gui-winui/           # C# WinUI 3 desktop (74 KB, 20 files)
│   ├── src/
│   │   ├── Views/       # MainWindow, Library, Reader, Settings
│   │   ├── Controls/    # PageView (SkiaSharp), MangaCover
│   │   ├── Services/    # IpcClient, CacheService
│   │   ├── Models/      # Manga, Page, ReadingProgress
│   │   └── ViewModels/  # MainViewModel
│   └── MangaReader.sln
├── gui-tauri/           # Tauri v2 + Svelte (111 KB, 29 files)
│   ├── src/             # Svelte frontend
│   │   ├── components/  # MangaCard, PageView, ReaderControls
│   │   ├── routes/      # Library, Reader, Settings
│   │   └── lib/stores/  # Reactive state management
│   └── src-tauri/       # Tauri Rust backend (17 IPC handlers)
├── ai-pipeline/         # Python AI sidecar
│   ├── src/
│   │   ├── vision/      # Gemini vision analysis
│   │   ├── tts/         # Kokoro TTS
│   │   ├── ocr/         # Manga OCR
│   │   └── llm/         # LLM integration
│   └── pyproject.toml
└── plugins/             # WASM plugin system (31 KB, 7 files)
    ├── traits/          # Plugin trait + register macro
    └── examples/        # Translator, Metadata Fetcher
```

## Feature Comparison

| Feature | v1 (Python/Tkinter) | v2 (Rust/C#/Tauri) |
|---------|---------------------|---------------------|
| Archive support | CBZ, CBR, PDF | CBZ, CBR, CB7, CBT, PDF, folders |
| Page rendering | PIL → Tkinter | GPU-accelerated (SkiaSharp / Canvas2D) |
| Panel detection | OpenCV (2 modes) | OpenCV + YOLO (via ONNX) |
| Reading modes | Single page | Single, Double, Webtoon, Panel-by-panel |
| Performance | Slow (Python) | Blazing fast (Rust zero-cost abstractions) |
| Library management | Manual file pick | Auto-scan, watch folders, 10k+ volumes |
| Search | None | Full-text via Tantivy (fuzzy, ranked) |
| Tracking | None | AniList, MyAnimeList, MAL, Kitsu |
| TTS | Kokoro only | Kokoro + ElevenLabs configurable |
| OCR | EasyOCR | manga-ocr + EasyOCR |
| Plugin system | None | WASM sandboxed plugins |
| Install size | ~200 MB | ~5 MB (Rust) + GUI deps |
| Cross-platform | Windows only | Windows, Mac, Linux |

## Build & Run

### Prerequisites
- Rust toolchain: `rustup target add wasm32-wasi`
- .NET 8 SDK (for WinUI)
- Node.js 18+ (for Tauri)
- Python 3.10+ (for AI pipeline)

### Rust Engine
```bash
cd engine
cargo build --release
cargo run --release -- --engine-dir ./data
```

### C# WinUI GUI (Windows)
```bash
cd gui-winui
dotnet build -c Release
dotnet run -c Release
```

### Tauri v2 GUI (Cross-platform)
```bash
cd gui-tauri
npm install
npx tauri dev
```

### Python AI Pipeline
```bash
cd ai-pipeline
uv venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
uv pip install -e .
python -m src.server
```

### Plugins
```bash
cd plugins/examples/translator
cargo build --target wasm32-wasi --release
```

## License
MIT
