//! JSON-RPC IPC server over local sockets / named pipes.
//!
//! Provides a bidirectional JSON-RPC 2.0 transport over platform-native
//! IPC mechanisms (Unix domain sockets on Linux/macOS, named pipes on
//! Windows). The server dispatches incoming method calls to the
//! appropriate engine subsystems.
//!
//! ## Protocol
//! Requests and responses are newline-delimited JSON-RPC 2.0 messages
//! over a stream-oriented transport.
//!
//! ## Methods
//! - `manga.list` — List all manga
//! - `manga.get` — Get manga details
//! - `manga.create` — Create a new manga entry
//! - `manga.update` — Update manga details
//! - `manga.delete` — Delete a manga
//! - `volume.list` — List volumes for a manga
//! - `volume.create` — Create a volume
//! - `chapter.list` — List chapters for a volume
//! - `chapter.create` — Create a chapter
//! - `page.list` — List pages for a chapter
//! - `bookmark.list` — List bookmarks for a manga
//! - `bookmark.create` — Create a bookmark
//! - `bookmark.delete` — Delete a bookmark
//! - `progress.get` — Get reading progress
//! - `progress.set` — Update reading progress
//! - `category.list` — List categories
//! - `category.create` — Create a category
//! - `category.assign` — Assign category to manga
//! - `category.unassign` — Remove category from manga
//! - `archive.extract` — Extract an archive file
//! - `panel.detect` — Detect panels in an image
//! - `search.query` — Full-text search
//! - `cache.clear` — Clear image cache
//! - `system.ping` — Health check
//! - `system.status` — Engine status

mod protocol;

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{Framed, LinesCodec};

use crate::EngineContext;

use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// Type alias for a shared reference to the engine context.
type Ctx = Arc<EngineContext>;

/// Start the IPC server and serve requests until shutdown.
pub async fn serve(ctx: EngineContext) -> Result<()> {
    let ctx = Arc::new(ctx);
    let endpoint = &ctx.config.ipc_endpoint;

    // We listen on a TCP loopback socket for simplicity and cross-platform compatibility.
    // On Windows, named pipes would be preferred; on Unix, Unix domain sockets.
    // For now, we use localhost TCP as the universal transport.
    let listener = TcpListener::bind(endpoint)
        .await
        .context(format!("Failed to bind IPC server to {}", endpoint))?;

    log::info!("IPC server listening on {}", endpoint);

    // Spawn the connection handler pool
    let mut next_id = 0u64;

    loop {
        let (stream, addr) = listener
            .accept()
            .await
            .context("Failed to accept IPC connection")?;

        log::info!("New IPC connection from: {}", addr);

        let ctx = ctx.clone();
        let id = next_id;
        next_id += 1;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(ctx, stream, id).await {
                log::error!("IPC connection {} error: {}", id, e);
            }
            log::info!("IPC connection {} closed", id);
        });
    }
}

/// Handle a single IPC connection: read JSON-RPC messages from the
/// framed stream, dispatch them, and write responses back.
async fn handle_connection(ctx: Ctx, stream: TcpStream, _id: u64) -> Result<()> {
    let mut framed = Framed::new(stream, LinesCodec::new_with_max_length(10 * 1024 * 1024));

    while let Some(line) = framed.next().await {
        let line = line.context("Failed to read line from IPC stream")?;

        // Parse the JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let err = JsonRpcError::parse_error().with_message(format!("Parse error: {}", e));
                let response = JsonRpcResponse::error(None, err);
                let resp_line =
                    serde_json::to_string(&response).context("Failed to serialize error response")?;
                framed.send(resp_line).await.ok();
                continue;
            }
        };

        // Dispatch the request
        let response = dispatch(&ctx, &request).await;

        // Send the response
        let resp_line = serde_json::to_string(&response)?;
        framed.send(resp_line).await?;
    }

    Ok(())
}

/// Dispatch a JSON-RPC request to the appropriate handler.
async fn dispatch(ctx: &Ctx, request: &JsonRpcRequest) -> JsonRpcResponse {
    let method = request.method.as_str();

    let result = match method {
        "system.ping" => handle_ping(ctx, request).await,
        "system.status" => handle_status(ctx, request).await,
        "manga.list" => handle_manga_list(ctx, request).await,
        "manga.get" => handle_manga_get(ctx, request).await,
        "manga.create" => handle_manga_create(ctx, request).await,
        "manga.update" => handle_manga_update(ctx, request).await,
        "manga.delete" => handle_manga_delete(ctx, request).await,
        "volume.list" => handle_volume_list(ctx, request).await,
        "volume.create" => handle_volume_create(ctx, request).await,
        "chapter.list" => handle_chapter_list(ctx, request).await,
        "chapter.create" => handle_chapter_create(ctx, request).await,
        "page.list" => handle_page_list(ctx, request).await,
        "bookmark.list" => handle_bookmark_list(ctx, request).await,
        "bookmark.create" => handle_bookmark_create(ctx, request).await,
        "bookmark.delete" => handle_bookmark_delete(ctx, request).await,
        "progress.get" => handle_progress_get(ctx, request).await,
        "progress.set" => handle_progress_set(ctx, request).await,
        "category.list" => handle_category_list(ctx, request).await,
        "category.create" => handle_category_create(ctx, request).await,
        "category.assign" => handle_category_assign(ctx, request).await,
        "category.unassign" => handle_category_unassign(ctx, request).await,
        "archive.extract" => handle_archive_extract(ctx, request).await,
        "panel.detect" => handle_panel_detect(ctx, request).await,
        "search.query" => handle_search_query(ctx, request).await,
        "cache.clear" => handle_cache_clear(ctx, request).await,
        _ => {
            let err = JsonRpcError::method_not_found()
                .with_message(format!("Method '{}' not found", method));
            return JsonRpcResponse::error(Some(request.id.clone()), err);
        }
    };

    match result {
        Ok(value) => JsonRpcResponse::success(Some(request.id.clone()), value),
        Err(e) => {
            let err = JsonRpcError::internal_error().with_message(format!("{}", e));
            JsonRpcResponse::error(Some(request.id.clone()), err)
        }
    }
}

// ─── Handler Implementations ───────────────────────────────────────

async fn handle_ping(_ctx: &Ctx, _req: &JsonRpcRequest) -> Result<serde_json::Value> {
    Ok(serde_json::json!({ "pong": true, "timestamp": chrono::Utc::now().to_rfc3339() }))
}

async fn handle_status(ctx: &Ctx, _req: &JsonRpcRequest) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "uptime": chrono::Utc::now().to_rfc3339(),
        "db_path": ctx.config.db_path,
        "cache_size": ctx.image_cache.total_size(),
        "cache_entries": ctx.image_cache.entry_count(),
        "gpu_enabled": ctx.config.gpu_enabled,
        "yolo_enabled": ctx.config.yolo_enabled,
    }))
}

async fn handle_manga_list(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let category_id = req.params.as_ref()
        .and_then(|p| p.get("category_id"))
        .and_then(|v| v.as_i64());

    let manga = ctx.db.list_manga(category_id)?;
    Ok(serde_json::to_value(manga)?)
}

async fn handle_manga_get(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let id = req.param_i64("id")?;
    let manga = ctx.db.get_manga(id)?;
    Ok(serde_json::to_value(manga)?)
}

async fn handle_manga_create(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let new_manga: crate::db::models::NewManga = req.parse_params()?;
    let id = ctx.db.insert_manga(&new_manga)?;
    Ok(serde_json::json!({ "id": id }))
}

async fn handle_manga_update(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let manga: crate::db::models::Manga = req.parse_params()?;
    ctx.db.update_manga(&manga)?;
    Ok(serde_json::json!({ "success": true }))
}

async fn handle_manga_delete(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let id = req.param_i64("id")?;
    let deleted = ctx.db.delete_manga(id)?;
    Ok(serde_json::json!({ "deleted": deleted }))
}

async fn handle_volume_list(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let manga_id = req.param_i64("manga_id")?;
    let volumes = ctx.db.list_volumes(manga_id)?;
    Ok(serde_json::to_value(volumes)?)
}

async fn handle_volume_create(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let new_volume: crate::db::models::NewVolume = req.parse_params()?;
    let id = ctx.db.insert_volume(&new_volume)?;
    Ok(serde_json::json!({ "id": id }))
}

async fn handle_chapter_list(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let volume_id = req.param_i64("volume_id")?;
    let chapters = ctx.db.list_chapters(volume_id)?;
    Ok(serde_json::to_value(chapters)?)
}

async fn handle_chapter_create(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let new_chapter: crate::db::models::NewChapter = req.parse_params()?;
    let id = ctx.db.insert_chapter(&new_chapter)?;
    Ok(serde_json::json!({ "id": id }))
}

async fn handle_page_list(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let chapter_id = req.param_i64("chapter_id")?;
    let pages = ctx.db.list_pages(chapter_id)?;
    Ok(serde_json::to_value(pages)?)
}

async fn handle_bookmark_list(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let manga_id = req.param_i64("manga_id")?;
    let bookmarks = ctx.db.list_bookmarks(manga_id)?;
    Ok(serde_json::to_value(bookmarks)?)
}

async fn handle_bookmark_create(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let new_bookmark: crate::db::models::NewBookmark = req.parse_params()?;
    let id = ctx.db.insert_bookmark(&new_bookmark)?;
    Ok(serde_json::json!({ "id": id }))
}

async fn handle_bookmark_delete(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let id = req.param_i64("id")?;
    let deleted = ctx.db.delete_bookmark(id)?;
    Ok(serde_json::json!({ "deleted": deleted }))
}

async fn handle_progress_get(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let user_id = req.param_str("user_id")?;
    let chapter_id = req.param_i64("chapter_id")?;
    let progress = ctx.db.get_progress(&user_id, chapter_id)?;
    Ok(serde_json::to_value(progress)?)
}

async fn handle_progress_set(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let progress: crate::db::models::NewProgress = req.parse_params()?;
    ctx.db.upsert_progress(&progress)?;
    Ok(serde_json::json!({ "success": true }))
}

async fn handle_category_list(ctx: &Ctx, _req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let categories = ctx.db.list_categories()?;
    Ok(serde_json::to_value(categories)?)
}

async fn handle_category_create(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let new_category: crate::db::models::NewCategory = req.parse_params()?;
    let id = ctx.db.insert_category(&new_category)?;
    Ok(serde_json::json!({ "id": id }))
}

async fn handle_category_assign(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let manga_id = req.param_i64("manga_id")?;
    let category_id = req.param_i64("category_id")?;
    ctx.db.assign_category(manga_id, category_id)?;
    Ok(serde_json::json!({ "success": true }))
}

async fn handle_category_unassign(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let manga_id = req.param_i64("manga_id")?;
    let category_id = req.param_i64("category_id")?;
    ctx.db.unassign_category(manga_id, category_id)?;
    Ok(serde_json::json!({ "success": true }))
}

async fn handle_archive_extract(_ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let path = req.param_str("path")?;
    let entries = crate::archive::extract(&path)?;
    let entry_info: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "path": e.path.to_string_lossy(),
                "page_index": e.page_index,
                "size_bytes": e.data.len(),
            })
        })
        .collect();
    Ok(serde_json::json!({ "entries": entry_info, "count": entries.len() }))
}

async fn handle_panel_detect(_ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let image_path = req.param_str("image_path")?;
    let img = crate::image::decode_from_path(&image_path)?;
    let detector = crate::panel::default_detector();
    let result = detector.detect(&img)?;
    Ok(serde_json::to_value(result)?)
}

async fn handle_search_query(ctx: &Ctx, req: &JsonRpcRequest) -> Result<serde_json::Value> {
    let query = req.param_str("query")?;
    let category_id = req.params.as_ref()
        .and_then(|p| p.get("category_id"))
        .and_then(|v| v.as_i64());
    let results = ctx.search.search(&query, category_id)?;
    Ok(serde_json::json!({ "results": results, "count": results.len() }))
}

async fn handle_cache_clear(ctx: &Ctx, _req: &JsonRpcRequest) -> Result<serde_json::Value> {
    ctx.image_cache.clear()?;
    Ok(serde_json::json!({ "success": true }))
}
