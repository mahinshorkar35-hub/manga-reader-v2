-- ====================================================================
-- Migration v3: FTS5 full-text search virtual table and triggers.
--
-- Creates an FTS5 virtual table over the manga table for fast
-- full-text search across title, alt_title, author, artist, and
-- description. Synchronization triggers keep the FTS index in sync
-- when manga rows are inserted, updated, or deleted.
-- ====================================================================

-- ─── FTS5 virtual table ─────────────────────────────────────────────
-- FTS5 builds an external content index over the manga table.
-- content='manga' means the index is synchronised with the manga table;
-- content_rowid='id' maps the FTS rowid to manga.id.
--
-- Columns indexed for search: title, alt_title, author, artist, description
-- The 'tokenize' option uses unicode61 which handles Unicode text well.
CREATE VIRTUAL TABLE IF NOT EXISTS manga_fts USING fts5(
    title,
    alt_title,
    author,
    artist,
    description,
    content='manga',
    content_rowid='id',
    tokenize='unicode61'
);

-- ─── Triggers to keep FTS index in sync ─────────────────────────────
-- After INSERT: add the new row to the FTS index.
CREATE TRIGGER IF NOT EXISTS trg_manga_fts_insert
    AFTER INSERT ON manga
    FOR EACH ROW
BEGIN
    INSERT INTO manga_fts (rowid, title, alt_title, author, artist, description)
    VALUES (NEW.id, NEW.title, NEW.alt_title, NEW.author, NEW.artist, NEW.description);
END;

-- After DELETE: remove the deleted row from the FTS index.
CREATE TRIGGER IF NOT EXISTS trg_manga_fts_delete
    AFTER DELETE ON manga
    FOR EACH ROW
BEGIN
    INSERT INTO manga_fts (manga_fts, rowid, title, alt_title, author, artist, description)
    VALUES ('delete', OLD.id, OLD.title, OLD.alt_title, OLD.author, OLD.artist, OLD.description);
END;

-- After UPDATE: remove the old row and insert the new one.
-- We use 'delete' + 'insert' strategy for correctness.
CREATE TRIGGER IF NOT EXISTS trg_manga_fts_update
    AFTER UPDATE ON manga
    FOR EACH ROW
BEGIN
    INSERT INTO manga_fts (manga_fts, rowid, title, alt_title, author, artist, description)
    VALUES ('delete', OLD.id, OLD.title, OLD.alt_title, OLD.author, OLD.artist, OLD.description);
    INSERT INTO manga_fts (rowid, title, alt_title, author, artist, description)
    VALUES (NEW.id, NEW.title, NEW.alt_title, NEW.author, NEW.artist, NEW.description);
END;

-- ─── Initial population: index all existing rows ────────────────────
-- This ensures that if this migration is applied to a database that
-- already has manga entries, they get indexed immediately.
INSERT INTO manga_fts (rowid, title, alt_title, author, artist, description)
SELECT id, title, alt_title, author, artist, description FROM manga;
