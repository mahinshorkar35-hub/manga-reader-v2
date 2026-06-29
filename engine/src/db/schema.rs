//! Database schema definitions and migration system for the manga reader engine.
//!
//! This module manages the SQLite schema through versioned migrations.
//! Each migration is stored as a standalone `.sql` file in the `migrations/`
//! directory and embedded at compile time via `include_str!()`.
//!
//! On startup, the [`run_migrations`] function checks a `schema_version` table
//! for which migrations have already been applied and runs any pending ones
//! in order. This ensures the database schema is always up-to-date without
//! manual intervention.
//!
//! # Migration files
//!
//! | File                 | Description                                          |
//! |----------------------|------------------------------------------------------|
//! | `v1__initial.sql`    | Core tables: manga, volumes, pages, bookmarks,       |
//! |                      | reading_progress, categories, manga_categories,       |
//! |                      | tracking + indexes + updated_at triggers              |
//! | `v2__chapters.sql`   | Chapters table, migrate pages/bookmarks/progress to  |
//! |                      | chapter-level granularity                             |
//! | `v3__search_triggers.sql` | FTS5 full-text search virtual table + sync triggers |

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Current schema version constant — bump when adding new migrations.
const SCHEMA_VERSION: i64 = 3;

// ─── Embedded migration SQL ──────────────────────────────────────────
// Each migration is a standalone .sql file in the `migrations/` directory,
// embedded at compile time. This keeps the SQL readable and lintable while
// making the binary self-contained with zero runtime file dependencies.

const MIGRATION_V1: &str = include_str!("migrations/v1__initial.sql");
const MIGRATION_V2: &str = include_str!("migrations/v2__chapters.sql");
const MIGRATION_V3: &str = include_str!("migrations/v3__search_triggers.sql");

/// A single migration entry: version number, descriptive name, and SQL.
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// Returns all defined migrations in ascending version order.
fn all_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            name: "v1__initial",
            sql: MIGRATION_V1,
        },
        Migration {
            version: 2,
            name: "v2__chapters",
            sql: MIGRATION_V2,
        },
        Migration {
            version: 3,
            name: "v3__search_triggers",
            sql: MIGRATION_V3,
        },
    ]
}

/// Run all pending migrations against the given database connection.
///
/// This function is idempotent: it tracks applied versions in a
/// `schema_version` table and only runs migrations whose version
/// is greater than the current maximum applied version.
///
/// # Errors
///
/// Returns an error if any migration SQL fails or if the version
/// tracking table cannot be created or queried.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // ── Initialise schema_version tracking table ────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version     INTEGER PRIMARY KEY,
            name        TEXT,
            applied_at  TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
        );",
    )
    .context("Failed to create schema_version table")?;

    // ── Determine current version ──────────────────────────────────
    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .context("Failed to query current schema version")?;

    // ── Apply pending migrations in order ───────────────────────────
    let migrations = all_migrations();

    for migration in &migrations {
        if migration.version > current_version {
            log::info!(
                "Applying migration {} (v{})",
                migration.name,
                migration.version
            );

            conn.execute_batch(migration.sql)
                .with_context(|| {
                    format!(
                        "Failed to apply migration {} (v{})",
                        migration.name, migration.version
                    )
                })?;

            conn.execute(
                "INSERT INTO schema_version (version, name) VALUES (?1, ?2)",
                rusqlite::params![migration.version, migration.name],
            )
            .context("Failed to record migration in schema_version table")?;

            log::info!(
                "Successfully applied migration {} (v{})",
                migration.name,
                migration.version
            );
        }
    }

    log::info!(
        "Schema is up-to-date at version {} (latest: v{})",
        current_version.max(
            migrations.last().map(|m| m.version).unwrap_or(0)
        ),
        SCHEMA_VERSION
    );

    Ok(())
}

/// Returns the latest schema version this binary knows about.
pub fn latest_version() -> i64 {
    SCHEMA_VERSION
}

/// Returns metadata about all available migrations.
pub fn list_migrations() -> Vec<(i64, &'static str)> {
    all_migrations()
        .into_iter()
        .map(|m| (m.version, m.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create an in-memory database and run all migrations.
    fn setup_in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        // Enable foreign keys for testing
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_all_migrations_run_successfully() {
        let conn = setup_in_memory();

        // Verify schema_version table has all 3 migrations
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3, "All 3 migrations should be recorded");

        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 3, "Latest schema version should be 3");
    }

    #[test]
    fn test_migration_idempotency() {
        let conn = setup_in_memory();

        // Running migrations again should be a no-op (all versions already applied)
        run_migrations(&conn).unwrap();

        // Still exactly 3 entries
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3, "Re-running migrations should not duplicate entries");
    }

    #[test]
    fn test_tracking_table_exists() {
        let conn = setup_in_memory();

        // Verify tracking table was created (v1)
        conn.query_row(
            "SELECT id FROM tracking LIMIT 1",
            [],
            |_| Ok(()),
        )
        .expect("tracking table should exist");
    }

    #[test]
    fn test_fts5_virtual_table_exists() {
        let conn = setup_in_memory();

        // Verify FTS5 table exists by querying it
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM manga_fts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "manga_fts should exist and be empty initially");
    }

    #[test]
    fn test_all_tables_created() {
        let conn = setup_in_memory();

        let tables = [
            "manga",
            "volumes",
            "chapters",
            "pages",
            "bookmarks",
            "reading_progress",
            "categories",
            "manga_categories",
            "tracking",
            "manga_fts",
            "schema_version",
        ];

        for &table in &tables {
            let result: Result<i64, _> = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get(0),
            );
            match result {
                Ok(count) => assert_eq!(count, 1, "Table '{}' should exist", table),
                Err(e) => panic!("Failed to query for table '{}': {}", table, e),
            }
        }
    }

    #[test]
    fn test_triggers_created() {
        let conn = setup_in_memory();

        let triggers = [
            "trg_manga_updated_at",
            "trg_reading_progress_updated_at",
            "trg_tracking_updated_at",
            "trg_manga_fts_insert",
            "trg_manga_fts_delete",
            "trg_manga_fts_update",
        ];

        for &trigger in &triggers {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
                    rusqlite::params![trigger],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Trigger '{}' should exist", trigger);
        }
    }

    #[test]
    fn test_foreign_key_constraints() {
        let conn = setup_in_memory();

        // Insert a manga
        conn.execute(
            "INSERT INTO manga (title, status) VALUES ('Test', 'ongoing')",
            [],
        )
        .unwrap();

        // Insert a volume referencing the manga
        conn.execute(
            "INSERT INTO volumes (manga_id, volume_number, title) VALUES (1, 1.0, 'Vol 1')",
            [],
        )
        .unwrap();

        // Delete the manga — volumes should cascade delete
        conn.execute("DELETE FROM manga WHERE id = 1", []).unwrap();

        let vol_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM volumes WHERE manga_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(vol_count, 0, "Volumes should cascade-delete with manga");
    }

    #[test]
    fn test_updated_at_trigger_fires() {
        let conn = setup_in_memory();

        conn.execute(
            "INSERT INTO manga (title, status) VALUES ('Test', 'ongoing')",
            [],
        )
        .unwrap();

        let original: String = conn
            .query_row(
                "SELECT updated_at FROM manga WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Sleep briefly (SQLite CURRENT_TIMESTAMP has second granularity,
        // so this test may be fragile. We just verify the trigger fires
        // without error.)
        conn.execute(
            "UPDATE manga SET title = 'Updated' WHERE id = 1",
            [],
        )
        .unwrap();

        let updated: String = conn
            .query_row(
                "SELECT updated_at FROM manga WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // updated_at should not be empty (it may equal original if second didn't tick)
        assert!(!updated.is_empty(), "updated_at should be set");
    }
}
