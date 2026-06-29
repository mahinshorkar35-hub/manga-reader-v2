//! Manga Reader Engine — Rust core engine library.
//!
//! This crate provides the core engine services for the manga reader:
//! - Archive extraction (CBZ, CBR, CB7, plain folders)
//! - Image decoding, caching, and resizing
//! - Panel detection and gutter-based splitting
//! - SQLite database (manga, volumes, pages, bookmarks, progress, categories)
//! - Tantivy full-text search
//! - JSON-RPC IPC server over local socket
//! - GPU-accelerated compute via wgpu (optional)
//! - ONNX Runtime YOLO inference (optional)

pub mod archive;
#[cfg(feature = "sqlite")]
pub mod db;
pub mod image;
pub mod ipc;
pub mod panel;
#[cfg(feature = "fts")]
pub mod search;

use std::sync::Arc;
use anyhow::Result;

/// Engine configuration loaded at startup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineConfig {
    /// Path to the SQLite database file.
    pub db_path: String,
    /// Path to the image cache directory.
    pub cache_dir: String,
    /// Path to the Tantivy index directory.
    pub index_dir: String,
    /// IPC socket path (Unix) or pipe name (Windows).
    pub ipc_endpoint: String,
    /// Maximum cache size in bytes (default: 512 MB).
    pub max_cache_size: u64,
    /// Whether to enable GPU compute.
    pub gpu_enabled: bool,
    /// Whether to enable YOLO inference.
    pub yolo_enabled: bool,
    /// Number of async worker threads.
    pub worker_threads: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            db_path: "manga_reader.db".to_string(),
            cache_dir: "cache".to_string(),
            index_dir: "index".to_string(),
            ipc_endpoint: "127.0.0.1:8500".to_string(),
            max_cache_size: 512 * 1024 * 1024,
            gpu_enabled: cfg!(feature = "gpu"),
            yolo_enabled: cfg!(feature = "yolo"),
            worker_threads: 4,
        }
    }
}

/// Initialize the engine with the given configuration.
/// Sets up logging, database, search index, cache, and IPC server.
pub async fn initialize(config: EngineConfig) -> Result<EngineContext> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Initializing manga reader engine");
    log::info!("  DB path:     {}", config.db_path);
    log::info!("  Cache dir:   {}", config.cache_dir);
    log::info!("  Index dir:   {}", config.index_dir);
    log::info!("  IPC endpoint: {}", config.ipc_endpoint);
    log::info!("  GPU enabled:  {}", config.gpu_enabled);
    log::info!("  YOLO enabled: {}", config.yolo_enabled);

    let ctx = EngineContext::new(config.clone()).await?;

    log::info!("Engine initialized successfully");
    Ok(ctx)
}

/// Engine runtime context holding all service handles.
#[derive(Clone)]
pub struct EngineContext {
    pub config: EngineConfig,
    #[cfg(feature = "sqlite")]
    pub db: Arc<db::Database>,
    #[cfg(feature = "fts")]
    pub search: Arc<search::SearchIndex>,
    pub image_cache: Arc<image::cache::ImageCache>,
}

impl EngineContext {
    pub async fn new(config: EngineConfig) -> anyhow::Result<Self> {
        // Initialize database
        #[cfg(feature = "sqlite")]
        let db = Arc::new(db::Database::open(&config.db_path)?);

        // Initialize search index
        #[cfg(feature = "fts")]
        let search = Arc::new(search::SearchIndex::open(&config.index_dir)?);

        // Initialize image cache
        let image_cache = Arc::new(image::cache::ImageCache::new(&config.cache_dir, config.max_cache_size)?);

        Ok(Self {
            config,
            #[cfg(feature = "sqlite")]
            db,
            #[cfg(feature = "fts")]
            search,
            image_cache,
        })
    }
}
