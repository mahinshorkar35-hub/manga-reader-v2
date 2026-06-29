-- ====================================================================
-- Migration v1: Initial schema
-- Core tables: manga, volumes, pages, bookmarks, reading_progress,
-- categories, manga_categories, tracking
-- Includes all constraints, indexes, foreign keys, updated_at triggers
-- ====================================================================

-- ─── Manga series table ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS manga (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    title           TEXT NOT NULL,
    alt_title       TEXT,
    author          TEXT,
    artist          TEXT,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'ongoing'
                    CHECK (status IN ('ongoing', 'completed', 'hiatus', 'cancelled', 'unknown')),
    cover_path      TEXT,
    source          TEXT,
    source_url      TEXT,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);

CREATE INDEX IF NOT EXISTS idx_manga_title ON manga(title);
CREATE INDEX IF NOT EXISTS idx_manga_status ON manga(status);
CREATE INDEX IF NOT EXISTS idx_manga_source ON manga(source);

-- ─── Volumes within a manga series ──────────────────────────────────
CREATE TABLE IF NOT EXISTS volumes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id        INTEGER NOT NULL,
    volume_number   REAL NOT NULL,
    title           TEXT NOT NULL,
    cover_path      TEXT,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_volumes_manga ON volumes(manga_id, volume_number);

-- ─── Pages within a volume (initially volume-linked) ────────────────
CREATE TABLE IF NOT EXISTS pages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    volume_id       INTEGER NOT NULL,
    page_number     INTEGER NOT NULL,
    image_path      TEXT NOT NULL,
    width           INTEGER,
    height          INTEGER,
    file_size       INTEGER,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (volume_id) REFERENCES volumes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pages_volume ON pages(volume_id, page_number);

-- ─── User bookmarks (initially volume-linked) ────────────────────────
CREATE TABLE IF NOT EXISTS bookmarks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id        INTEGER NOT NULL,
    volume_id       INTEGER,
    page_number     INTEGER NOT NULL,
    note            TEXT,
    user_id         TEXT NOT NULL DEFAULT 'default',
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE,
    FOREIGN KEY (volume_id) REFERENCES volumes(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_manga ON bookmarks(manga_id, user_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_user ON bookmarks(user_id);

-- ─── Reading progress (per-user per-volume, initially volume-linked) ─
CREATE TABLE IF NOT EXISTS reading_progress (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         TEXT NOT NULL,
    manga_id        INTEGER NOT NULL,
    volume_id       INTEGER NOT NULL,
    page_number     INTEGER NOT NULL DEFAULT 0,
    percentage      REAL NOT NULL DEFAULT 0.0
                    CHECK (percentage >= 0.0 AND percentage <= 100.0),
    completed       INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE,
    FOREIGN KEY (volume_id) REFERENCES volumes(id) ON DELETE CASCADE,
    UNIQUE(user_id, volume_id)
);

CREATE INDEX IF NOT EXISTS idx_progress_user_manga ON reading_progress(user_id, manga_id);
CREATE INDEX IF NOT EXISTS idx_progress_user ON reading_progress(user_id);

-- ─── Categories / tags for organizing manga ─────────────────────────
CREATE TABLE IF NOT EXISTS categories (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    color           TEXT DEFAULT '#6366f1',
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);

-- ─── Many-to-many: manga <-> categories ─────────────────────────────
CREATE TABLE IF NOT EXISTS manga_categories (
    manga_id        INTEGER NOT NULL,
    category_id     INTEGER NOT NULL,
    PRIMARY KEY (manga_id, category_id),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
);

-- ─── External tracking (MAL, Anilist, etc.) ─────────────────────────
CREATE TABLE IF NOT EXISTS tracking (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id        INTEGER NOT NULL,
    tracker_source  TEXT NOT NULL,
    tracker_id      TEXT,
    tracker_url     TEXT,
    score           REAL CHECK (score IS NULL OR (score >= 0 AND score <= 10)),
    chapters_read   INTEGER DEFAULT 0,
    volumes_read    INTEGER DEFAULT 0,
    start_date      TEXT,
    finish_date     TEXT,
    reread_count    INTEGER DEFAULT 0,
    notes           TEXT,
    last_synced     TEXT,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE,
    UNIQUE(manga_id, tracker_source)
);

CREATE INDEX IF NOT EXISTS idx_tracking_manga ON tracking(manga_id);
CREATE INDEX IF NOT EXISTS idx_tracking_source ON tracking(tracker_source);

-- ─── updated_at triggers ────────────────────────────────────────────
-- SQLite does not support per-column triggers, so these fire on ANY
-- UPDATE of the row and set updated_at to the current timestamp.

CREATE TRIGGER IF NOT EXISTS trg_manga_updated_at
    AFTER UPDATE ON manga
    FOR EACH ROW
BEGIN
    UPDATE manga SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_reading_progress_updated_at
    AFTER UPDATE ON reading_progress
    FOR EACH ROW
BEGIN
    UPDATE reading_progress SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_tracking_updated_at
    AFTER UPDATE ON tracking
    FOR EACH ROW
BEGIN
    UPDATE tracking SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;
