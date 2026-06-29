# Manga Reader v2 — Tauri + Svelte GUI

A cross-platform desktop manga reader built with **Tauri v2** and **Svelte**, serving as an alternative to the Windows-native WinUI version.

## Architecture

```
gui-tauri/
├── package.json              # Svelte + Vite + Tauri dependencies
├── vite.config.js            # Vite configuration
├── svelte.config.js          # Svelte configuration
├── index.html                # HTML entry point
├── src/
│   ├── main.js               # Svelte app bootstrap
│   ├── App.svelte            # Root component with sidebar routing
│   ├── app.css               # Dark/light theme CSS custom properties
│   ├── lib/stores/
│   │   └── index.js          # Centralized Svelte stores (theme, nav, library, reader, settings)
│   ├── routes/
│   │   ├── Library.svelte    # Browse, filter, search manga series
│   │   ├── Reader.svelte     # Full-screen manga reader with chapter selection
│   │   └── Settings.svelte   # App configuration (appearance, reading, TTS, API keys)
│   └── components/
│       ├── MangaCard.svelte  # Series card with cover, status badge, metadata
│       ├── PageView.svelte   # Canvas-based GPU-rendered page viewer
│       ├── SearchBar.svelte  # Search input with clear button
│       ├── ThemeToggle.svelte# Dark/light theme switcher
│       └── ReaderControls.svelte # Navigation bar (page jump, fit mode, direction, etc.)
└── src-tauri/
    ├── Cargo.toml            # Rust dependencies (Tauri 2, image, serde, etc.)
    ├── tauri.conf.json       # Tauri app config (window, security, bundle)
    ├── build.rs              # Tauri build script
    ├── capabilities/
    │   └── default.json      # Tauri v2 permission capabilities
    ├── icons/                # App icons
    └── src/
        ├── main.rs           # Binary entry point
        └── lib.rs            # Library with all IPC handlers & data models
```

## Key Design Decisions

### Rust Backend (Engine)
- **All data operations** go through the Rust engine via Tauri IPC — no Python dependency
- Commands: `get_library`, `get_library_stats`, `get_series`, `scan_manga_directory`, `get_chapters`, `get_chapter`, `get_page_image`, `search_series`, `get_settings`, `save_settings`, `reset_settings`
- State is persisted to the app data directory (`settings.json`, `library.json`)
- Image loading is handled server-side with base64 encoding for the frontend

### Svelte Frontend
- **Canvas-based rendering** (`PageView.svelte`) uses a 2D canvas with `desynchronized: true` for GPU-accelerated page display
- Three fit modes: fit-width, fit-height, original size
- Keyboard navigation (arrows, space, page up/down, home/end)
- Left/right click navigation (respects reading direction)
- Dark/light theme via CSS custom properties on `[data-theme]`

### Feature Parity with WinUI
| Feature | WinUI | Tauri v2 |
|---------|-------|----------|
| Library browsing | ✓ | ✓ |
| Status filtering | ✓ | ✓ |
| Search | ✓ | ✓ |
| Canvas page viewer | ✓ | ✓ (GPU-accelerated) |
| Chapter navigation | ✓ | ✓ |
| Page fit modes | ✓ | ✓ |
| RTL/LTR direction | ✓ | ✓ |
| Double page mode | ✓ | ✓ |
| Page number overlay | ✓ | ✓ |
| Settings persistence | ✓ | ✓ |
| TTS configuration | ✓ | ✓ |
| API key management | ✓ | ✓ |
| Directory scanning | ✓ | ✓ |
| Dark/light theme | ✓ | ✓ |

## Development

### Prerequisites
- Node.js >= 18
- Rust (stable)
- Tauri CLI v2

### Setup

```bash
cd gui-tauri
npm install
```

### Run in development mode

```bash
npm run tauri:dev
```

### Build for production

```bash
npm run tauri:build
```

## Data Flow

```
User Action → Svelte Component → invoke() IPC → Rust Command
    ↑                                                    |
    |                                                    ↓
User sees result ←─── Svelte reactive store ←─── JSON Response
```

The Rust backend manages all data, including:
- Scanning directories for manga series
- Reading and caching chapter images
- Persisting settings to disk
- Searching and filtering the library

The frontend is a pure consumer — it never accesses the filesystem directly.
