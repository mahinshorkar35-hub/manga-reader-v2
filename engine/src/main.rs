//! Manga Reader Engine — IPC Server Entry Point.
//!
//! This binary starts a JSON-RPC IPC server over a named pipe (Windows)
//! or Unix domain socket (Linux/macOS). The server listens for incoming
//! JSON-RPC requests and dispatches them to the engine subsystems.

use anyhow::Result;
use manga_reader_engine::{initialize, EngineConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from environment / config file
    let config = load_config()?;

    // Initialize the engine (DB, search, cache, GPU, YOLO)
    let ctx = initialize(config).await?;

    log::info!("Starting IPC server on endpoint: {}", ctx.config.ipc_endpoint);

    // Start the JSON-RPC IPC server
    // This blocks until the server shuts down.
    manga_reader_engine::ipc::serve(ctx).await?;

    log::info!("Engine shut down gracefully");
    Ok(())
}

/// Load configuration from environment variables or a config file.
fn load_config() -> Result<EngineConfig> {
    // Try to load from a JSON config file first
    let config_path = std::env::var("MANGA_ENGINE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("engine_config.json"));

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: EngineConfig = serde_json::from_str(&content)?;
        log::info!("Loaded config from: {}", config_path.display());
        return Ok(config);
    }

    // Fall back to environment variables with defaults
    Ok(EngineConfig {
        db_path: std::env::var("MANGA_DB_PATH").unwrap_or_else(|_| "manga_reader.db".into()),
        cache_dir: std::env::var("MANGA_CACHE_DIR").unwrap_or_else(|_| "cache".into()),
        index_dir: std::env::var("MANGA_INDEX_DIR").unwrap_or_else(|_| "index".into()),
        ipc_endpoint: std::env::var("MANGA_IPC_ENDPOINT")
            .unwrap_or_else(|_| "\\\\.\\pipe\\manga-reader-engine".into()),
        max_cache_size: std::env::var("MANGA_MAX_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(512 * 1024 * 1024),
        gpu_enabled: std::env::var("MANGA_GPU_ENABLED")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(true),
        yolo_enabled: std::env::var("MANGA_YOLO_ENABLED")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(true),
        worker_threads: std::env::var("MANGA_WORKER_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4),
    })
}
