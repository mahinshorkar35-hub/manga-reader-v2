//! SQLite database module.
//!
//! Provides the data access layer for the manga reader engine using
//! SQLite (via `rusqlite`). The schema supports:
//! - **Manga** series catalog
//! - **Volumes** within a series
//! - **Chapters** within a volume
//! - **Pages** within a chapter
//! - **Bookmarks** (user-saved positions)
//! - **Reading progress** (per-user/chapter)
//! - **Categories** for organizing manga
//! - **Manga-Category** many-to-many relationship

pub mod schema;
pub mod models;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

use models::*;

/// Thread-safe wrapper around a SQLite connection.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (or create) a SQLite database at the given path.
    ///
    /// Runs all migrations to ensure the schema is up-to-date.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .context(format!("Failed to open database at: {}", path.as_ref().display()))?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("Failed to enable WAL mode")?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .context("Failed to enable foreign keys")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Run migrations
        db.run_migrations()?;

        log::info!("Database opened successfully at: {}", path.as_ref().display());
        Ok(db)
    }

    /// Run all pending migrations.
    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        schema::run_migrations(&conn)
            .context("Failed to run database migrations")?;
        Ok(())
    }

    // ─── Manga CRUD ────────────────────────────────────────────────

    /// Insert a new manga entry.
    pub fn insert_manga(&self, manga: &NewManga) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO manga (title, alt_title, author, artist, description, status, cover_path, source, source_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                manga.title,
                manga.alt_title,
                manga.author,
                manga.artist,
                manga.description,
                manga.status,
                manga.cover_path,
                manga.source,
                manga.source_url,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get a manga by its ID.
    pub fn get_manga(&self, id: i64) -> Result<Option<Manga>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, alt_title, author, artist, description, status, cover_path, source, source_url, created_at, updated_at
             FROM manga WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Manga::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// Get all manga, optionally filtered by category.
    pub fn list_manga(&self, category_id: Option<i64>) -> Result<Vec<Manga>> {
        let conn = self.conn.lock().unwrap();

        let sql = match category_id {
            Some(_) => {
                "SELECT m.id, m.title, m.alt_title, m.author, m.artist, m.description, m.status,
                        m.cover_path, m.source, m.source_url, m.created_at, m.updated_at
                 FROM manga m
                 JOIN manga_categories mc ON m.id = mc.manga_id
                 WHERE mc.category_id = ?1
                 ORDER BY m.title"
            }
            None => {
                "SELECT id, title, alt_title, author, artist, description, status,
                        cover_path, source, source_url, created_at, updated_at
                 FROM manga ORDER BY title"
            }
        };

        let mut stmt = conn.prepare(sql)?;

        let rows: Box<dyn Iterator<Item = rusqlite::Result<Manga>>> = match category_id {
            Some(cid) => Box::new(stmt.query_map(params![cid], |row| Manga::from_row(row))?),
            None => Box::new(stmt.query_map([], |row| Manga::from_row(row))?),
        };

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Update a manga entry.
    pub fn update_manga(&self, manga: &Manga) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE manga SET title=?1, alt_title=?2, author=?3, artist=?4, description=?5,
             status=?6, cover_path=?7, source=?8, source_url=?9, updated_at=CURRENT_TIMESTAMP
             WHERE id=?10",
            params![
                manga.title,
                manga.alt_title,
                manga.author,
                manga.artist,
                manga.description,
                manga.status,
                manga.cover_path,
                manga.source,
                manga.source_url,
                manga.id,
            ],
        )?;
        Ok(())
    }

    /// Delete a manga and all associated data.
    pub fn delete_manga(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM manga WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ─── Volume CRUD ───────────────────────────────────────────────

    /// Insert a new volume.
    pub fn insert_volume(&self, volume: &NewVolume) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO volumes (manga_id, volume_number, title, cover_path)
             VALUES (?1, ?2, ?3, ?4)",
            params![volume.manga_id, volume.volume_number, volume.title, volume.cover_path],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List volumes for a manga, ordered by volume number.
    pub fn list_volumes(&self, manga_id: i64) -> Result<Vec<Volume>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, manga_id, volume_number, title, cover_path, created_at
             FROM volumes WHERE manga_id = ?1 ORDER BY volume_number",
        )?;

        let rows = stmt.query_map(params![manga_id], |row| Volume::from_row(row))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    // ─── Chapter CRUD ──────────────────────────────────────────────

    /// Insert a new chapter.
    pub fn insert_chapter(&self, chapter: &NewChapter) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO chapters (volume_id, manga_id, chapter_number, title, page_count, archive_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                chapter.volume_id,
                chapter.manga_id,
                chapter.chapter_number,
                chapter.title,
                chapter.page_count,
                chapter.archive_path,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List chapters for a volume.
    pub fn list_chapters(&self, volume_id: i64) -> Result<Vec<Chapter>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, volume_id, manga_id, chapter_number, title, page_count, archive_path, created_at
             FROM chapters WHERE volume_id = ?1 ORDER BY chapter_number",
        )?;

        let rows = stmt.query_map(params![volume_id], |row| Chapter::from_row(row))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    // ─── Page CRUD ─────────────────────────────────────────────────

    /// Insert a page.
    pub fn insert_page(&self, page: &NewPage) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO pages (chapter_id, page_number, image_path, width, height, file_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                page.chapter_id,
                page.page_number,
                page.image_path,
                page.width,
                page.height,
                page.file_size,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List pages for a chapter, ordered by page number.
    pub fn list_pages(&self, chapter_id: i64) -> Result<Vec<Page>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, chapter_id, page_number, image_path, width, height, file_size, created_at
             FROM pages WHERE chapter_id = ?1 ORDER BY page_number",
        )?;

        let rows = stmt.query_map(params![chapter_id], |row| Page::from_row(row))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    // ─── Bookmark CRUD ─────────────────────────────────────────────

    /// Insert a bookmark.
    pub fn insert_bookmark(&self, bookmark: &NewBookmark) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO bookmarks (manga_id, chapter_id, page_number, note, user_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                bookmark.manga_id,
                bookmark.chapter_id,
                bookmark.page_number,
                bookmark.note,
                bookmark.user_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List bookmarks for a manga.
    pub fn list_bookmarks(&self, manga_id: i64) -> Result<Vec<Bookmark>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, manga_id, chapter_id, page_number, note, user_id, created_at
             FROM bookmarks WHERE manga_id = ?1 ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![manga_id], |row| Bookmark::from_row(row))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Delete a bookmark.
    pub fn delete_bookmark(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ─── Reading Progress ──────────────────────────────────────────

    /// Upsert reading progress for a user on a chapter.
    pub fn upsert_progress(&self, progress: &NewProgress) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO progress (user_id, manga_id, chapter_id, page_number, percentage, completed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(user_id, chapter_id) DO UPDATE SET
                page_number = excluded.page_number,
                percentage = excluded.percentage,
                completed = excluded.completed,
                updated_at = CURRENT_TIMESTAMP",
            params![
                progress.user_id,
                progress.manga_id,
                progress.chapter_id,
                progress.page_number,
                progress.percentage,
                progress.completed,
            ],
        )?;
        Ok(())
    }

    /// Get reading progress for a user on a specific chapter.
    pub fn get_progress(&self, user_id: &str, chapter_id: i64) -> Result<Option<Progress>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, manga_id, chapter_id, page_number, percentage, completed, created_at, updated_at
             FROM progress WHERE user_id = ?1 AND chapter_id = ?2",
        )?;

        let mut rows = stmt.query(params![user_id, chapter_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Progress::from_row(row)?)),
            None => Ok(None),
        }
    }

    // ─── Categories ────────────────────────────────────────────────

    /// Insert a new category.
    pub fn insert_category(&self, category: &NewCategory) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO categories (name, description, color) VALUES (?1, ?2, ?3)",
            params![category.name, category.description, category.color],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List all categories.
    pub fn list_categories(&self) -> Result<Vec<Category>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, color, created_at FROM categories ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| Category::from_row(row))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Assign a category to a manga.
    pub fn assign_category(&self, manga_id: i64, category_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO manga_categories (manga_id, category_id) VALUES (?1, ?2)",
            params![manga_id, category_id],
        )?;
        Ok(())
    }

    /// Remove a category from a manga.
    pub fn unassign_category(&self, manga_id: i64, category_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM manga_categories WHERE manga_id = ?1 AND category_id = ?2",
            params![manga_id, category_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_query_manga() {
        let db = Database::open(":memory:").unwrap();

        let new = NewManga {
            title: "Test Manga".into(),
            alt_title: None,
            author: Some("Author".into()),
            artist: None,
            description: Some("A test manga".into()),
            status: "ongoing".into(),
            cover_path: None,
            source: Some("local".into()),
            source_url: None,
        };

        let id = db.insert_manga(&new).unwrap();
        assert!(id > 0);

        let manga = db.get_manga(id).unwrap().unwrap();
        assert_eq!(manga.title, "Test Manga");

        let list = db.list_manga(None).unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_progress_upsert() {
        let db = Database::open(":memory:").unwrap();

        // First insert manga, volume, chapter
        let manga_id = db.insert_manga(&NewManga {
            title: "Manga".into(),
            alt_title: None,
            author: None,
            artist: None,
            description: None,
            status: "ongoing".into(),
            cover_path: None,
            source: None,
            source_url: None,
        }).unwrap();

        let vol_id = db.insert_volume(&NewVolume {
            manga_id,
            volume_number: 1.0,
            title: "Vol 1".into(),
            cover_path: None,
        }).unwrap();

        let ch_id = db.insert_chapter(&NewChapter {
            volume_id: vol_id,
            manga_id,
            chapter_number: 1.0,
            title: "Ch 1".into(),
            page_count: 20,
            archive_path: "/path/to/ch1.cbz".into(),
        }).unwrap();

        // Upsert progress
        db.upsert_progress(&NewProgress {
            user_id: "user1".into(),
            manga_id,
            chapter_id: ch_id,
            page_number: 5,
            percentage: 25.0,
            completed: false,
        }).unwrap();

        let prog = db.get_progress("user1", ch_id).unwrap().unwrap();
        assert_eq!(prog.page_number, 5);
        assert!((prog.percentage - 25.0).abs() < 0.001);
    }
}
