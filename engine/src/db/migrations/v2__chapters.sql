-- ====================================================================
-- Migration v2: Add chapters table and migrate pages, bookmarks, and
-- reading_progress to support chapter-level granularity.
--
-- This migration:
--   1. Creates the chapters table
--   2. Adds chapter_id columns to pages, bookmarks, reading_progress
--   3. Recreates reading_progress to switch its UNIQUE constraint
--      from (user_id, volume_id) to (user_id, chapter_id)
--   4. Adds appropriate indexes
-- ====================================================================

-- ─── Chapters table ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS chapters (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    volume_id       INTEGER NOT NULL,
    manga_id        INTEGER NOT NULL,
    chapter_number  REAL NOT NULL,
    title           TEXT,
    page_count      INTEGER DEFAULT 0,
    archive_path    TEXT,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (volume_id) REFERENCES volumes(id) ON DELETE CASCADE,
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chapters_volume ON chapters(volume_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_chapters_manga ON chapters(manga_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_chapters_manga_number ON chapters(manga_id, chapter_number);

-- ─── Link pages to chapters ─────────────────────────────────────────
ALTER TABLE pages ADD COLUMN chapter_id INTEGER REFERENCES chapters(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_pages_chapter ON pages(chapter_id, page_number);

-- Update existing pages: if volume_id maps to a single chapter, link them.
-- This is a best-effort backfill; new inserts should set chapter_id directly.
UPDATE pages
SET chapter_id = (
    SELECT id FROM chapters
    WHERE chapters.volume_id = pages.volume_id
    LIMIT 1
)
WHERE pages.chapter_id IS NULL;

-- ─── Link bookmarks to chapters ─────────────────────────────────────
ALTER TABLE bookmarks ADD COLUMN chapter_id INTEGER REFERENCES chapters(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_bookmarks_chapter ON bookmarks(chapter_id);

-- ─── Recreate reading_progress with chapter-level uniqueness ────────
-- SQLite does not support DROP CONSTRAINT / ALTER COLUMN, so we must
-- create a new table, copy data, drop the old one, and rename.

-- 1. Create the new table with the desired schema
CREATE TABLE IF NOT EXISTS reading_progress_new (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         TEXT NOT NULL,
    manga_id        INTEGER NOT NULL,
    volume_id       INTEGER,
    chapter_id      INTEGER,
    page_number     INTEGER NOT NULL DEFAULT 0,
    percentage      REAL NOT NULL DEFAULT 0.0
                    CHECK (percentage >= 0.0 AND percentage <= 100.0),
    completed       INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    FOREIGN KEY (manga_id) REFERENCES manga(id) ON DELETE CASCADE,
    FOREIGN KEY (volume_id) REFERENCES volumes(id) ON DELETE SET NULL,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE,
    UNIQUE(user_id, chapter_id)
);

-- 2. Copy existing data
INSERT INTO reading_progress_new (
    id, user_id, manga_id, volume_id, chapter_id,
    page_number, percentage, completed, created_at, updated_at
)
SELECT
    rp.id, rp.user_id, rp.manga_id, rp.volume_id, NULL,
    rp.page_number, rp.percentage, rp.completed, rp.created_at, rp.updated_at
FROM reading_progress rp;

-- 3. Drop old table
DROP TABLE IF EXISTS reading_progress;

-- 4. Rename new table
ALTER TABLE reading_progress_new RENAME TO reading_progress;

-- 5. Recreate indexes
CREATE INDEX IF NOT EXISTS idx_progress_user ON reading_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_progress_user_manga ON reading_progress(user_id, manga_id);
CREATE INDEX IF NOT EXISTS idx_progress_chapter ON reading_progress(chapter_id);

-- 6. Recreate the updated_at trigger (was dropped with the old table)
CREATE TRIGGER IF NOT EXISTS trg_reading_progress_updated_at
    AFTER UPDATE ON reading_progress
    FOR EACH ROW
BEGIN
    UPDATE reading_progress SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;
