//! Full-text search engine using Tantivy.
//!
//! Provides indexing and searching for manga metadata including
//! titles, authors, descriptions, and category names.
//!
//! ## Index Schema
//! - `id` (stored, u64) — database record ID
//! - `type` (stored, string) — record type: "manga", "volume", "chapter"
//! - `title` (text, indexed) — title field for full-text search
//! - `author` (text, indexed) — author name
//! - `description` (text, indexed) — description text
//! - `category` (text, indexed) — category/tag names
//! - `manga_id` (stored, u64) — parent manga ID for filtering
//! - `source` (stored, string) — source name

use anyhow::{Context, Result};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

/// Search result from a Tantivy query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    /// The database record ID.
    pub id: u64,
    /// The record type ("manga", "volume", "chapter").
    pub record_type: String,
    /// The matched title.
    pub title: String,
    /// Matched author (if applicable).
    pub author: Option<String>,
    /// The score of this match.
    pub score: f64,
    /// Parent manga ID (for filtering).
    pub manga_id: Option<u64>,
}

/// The full-text search index wrapper.
pub struct SearchIndex {
    schema: Schema,
    index: Index,
    reader: IndexReader,
    writer: std::sync::Mutex<IndexWriter>,
    /// Field definitions for internal use.
    fields: SearchFields,
}

/// Internal field references for the Tantivy schema.
struct SearchFields {
    id: Field,
    record_type: Field,
    title: Field,
    author: Field,
    description: Field,
    category: Field,
    manga_id: Field,
    source: Field,
}

impl SearchIndex {
    /// Open (or create) a Tantivy search index at the given directory path.
    pub fn open<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let path = index_path.as_ref();
        std::fs::create_dir_all(path).context("Failed to create search index directory")?;

        let mut schema_builder = Schema::builder();

        let fields = SearchFields {
            id: schema_builder.add_u64_field("id", STORED | INDEXED),
            record_type: schema_builder.add_text_field("type", STRING | STORED),
            title: schema_builder.add_text_field("title", TEXT | STORED),
            author: schema_builder.add_text_field("author", TEXT | STORED),
            description: schema_builder.add_text_field("description", TEXT),
            category: schema_builder.add_text_field("category", TEXT),
            manga_id: schema_builder.add_u64_field("manga_id", STORED | INDEXED),
            source: schema_builder.add_text_field("source", STRING | STORED),
        };

        let schema = schema_builder.build();

        let index = Index::create_in_dir(path, schema.clone())
            .context("Failed to create Tantivy index directory")?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create Tantivy reader")?;

        let writer = IndexWriter::new(&index)
            .context("Failed to create Tantivy writer")?;

        log::info!("Search index opened at: {}", path.display());

        Ok(Self {
            schema,
            index,
            reader,
            writer: std::sync::Mutex::new(writer),
            fields,
        })
    }

    /// Index a manga entry for full-text search.
    pub fn index_manga(
        &self,
        id: u64,
        title: &str,
        author: Option<&str>,
        description: Option<&str>,
        categories: &[String],
        source: Option<&str>,
    ) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();

        // Build category string from list
        let category_str = categories.join(" ");

        let mut doc = TantivyDocument::new();
        doc.add_u64(self.fields.id, id);
        doc.add_text(self.fields.record_type, "manga");
        doc.add_text(self.fields.title, title);
        if let Some(a) = author {
            doc.add_text(self.fields.author, a);
        }
        if let Some(d) = description {
            doc.add_text(self.fields.description, d);
        }
        if !category_str.is_empty() {
            doc.add_text(self.fields.category, &category_str);
        }
        doc.add_u64(self.fields.manga_id, id);
        if let Some(s) = source {
            doc.add_text(self.fields.source, s);
        }

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Index a volume entry.
    pub fn index_volume(
        &self,
        id: u64,
        manga_id: u64,
        title: &str,
        volume_number: f64,
    ) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();

        let mut doc = TantivyDocument::new();
        doc.add_u64(self.fields.id, id);
        doc.add_text(self.fields.record_type, "volume");
        doc.add_text(self.fields.title, &format!("Volume {}: {}", volume_number, title));
        doc.add_u64(self.fields.manga_id, manga_id);

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Index a chapter entry.
    pub fn index_chapter(
        &self,
        id: u64,
        manga_id: u64,
        title: &str,
        chapter_number: f64,
    ) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();

        let mut doc = TantivyDocument::new();
        doc.add_u64(self.fields.id, id);
        doc.add_text(self.fields.record_type, "chapter");
        doc.add_text(self.fields.title, &format!("Chapter {}: {}", chapter_number, title));
        doc.add_u64(self.fields.manga_id, manga_id);

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Remove a document by ID.
    pub fn remove_document(&self, id: u64) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        let term = tantivy::Term::of_field(self.fields.id, id);
        writer.delete_term(term);
        writer.commit()?;
        Ok(())
    }

    /// Search the index with a full-text query.
    ///
    /// Returns up to `limit` results, optionally filtered by `manga_id`.
    pub fn search(&self, query_str: &str, manga_id: Option<i64>) -> Result<Vec<SearchResult>> {
        let reader = self.reader.clone();
        let searcher = reader.searcher();

        // Build a query parser that searches across title, author, description, category
        let mut query_parser = QueryParser::for_index(&self.index, vec![
            self.fields.title,
            self.fields.author,
            self.fields.description,
            self.fields.category,
        ]);

        // Enable fuzzy matching with 1 edit distance
        query_parser.set_field_fuzzy(self.fields.title, true, 1, 3);

        let query = query_parser
            .parse_query(query_str)
            .context(format!("Failed to parse search query: '{}'", query_str))?;

        // Apply manga_id filter if specified
        let query = if let Some(mid) = manga_id {
            let term = tantivy::Term::of_field(self.fields.manga_id, mid as u64);
            let term_query = tantivy::query::TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);
            let subqueries: Vec<(f64, Box<dyn tantivy::query::Query>)> = vec![
                (1.0, Box::new(query)),
                (0.0, Box::new(term_query)),
            ];
            Box::new(tantivy::query::BooleanQuery::new(subqueries)) as Box<dyn tantivy::query::Query>
        } else {
            Box::new(query) as Box<dyn tantivy::query::Query>
        };

        // Collect top 50 results
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(50))
            .context("Search execution failed")?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc::<TantivyDocument>(doc_address)
                .context("Failed to retrieve search document")?;

            let id = doc
                .get_first(self.fields.id)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let record_type = doc
                .get_first(self.fields.record_type)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let title = doc
                .get_first(self.fields.title)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let author = doc
                .get_first(self.fields.author)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let doc_manga_id = doc
                .get_first(self.fields.manga_id)
                .and_then(|v| v.as_u64());

            results.push(SearchResult {
                id,
                record_type,
                title,
                author,
                score: score as f64,
                manga_id: doc_manga_id,
            });
        }

        Ok(results)
    }

    /// Get the total number of indexed documents.
    pub fn document_count(&self) -> Result<usize> {
        let reader = self.reader.clone();
        let searcher = reader.searcher();
        Ok(searcher.num_docs() as usize)
    }

    /// Clear all indexed documents.
    pub fn clear(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.delete_all_documents()?;
        writer.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_and_search() {
        let temp_dir = TempDir::new().unwrap();
        let index = SearchIndex::open(temp_dir.path()).unwrap();

        // Index a manga
        index
            .index_manga(
                1,
                "One Piece",
                Some("Oda"),
                Some("A pirate adventure"),
                &["Shonen".to_string(), "Adventure".to_string()],
                Some("MangaPlus"),
            )
            .unwrap();

        // Index another manga
        index
            .index_manga(
                2,
                "Naruto",
                Some("Kishimoto"),
                Some("A ninja story"),
                &["Shonen".to_string()],
                Some("MangaPlus"),
            )
            .unwrap();

        // Search for "Pirate"
        let results = index.search("Pirate", None).unwrap();
        assert!(!results.is_empty(), "Should find 'One Piece' by 'Pirate'");
        let has_one_piece = results.iter().any(|r| r.title == "One Piece");
        assert!(has_one_piece, "Should find One Piece");

        // Search for "ninja"
        let results = index.search("ninja", None).unwrap();
        assert!(!results.is_empty(), "Should find 'Naruto' by 'ninja'");
        let has_naruto = results.iter().any(|r| r.title == "Naruto");
        assert!(has_naruto, "Should find Naruto");

        // Search with manga_id filter
        let results = index.search("Shonen", Some(1)).unwrap();
        let all_manga_1 = results.iter().all(|r| r.manga_id == Some(1));
        assert!(all_manga_1, "All results should have manga_id=1");
    }

    #[test]
    fn test_remove_document() {
        let temp_dir = TempDir::new().unwrap();
        let index = SearchIndex::open(temp_dir.path()).unwrap();

        index
            .index_manga(1, "Test Manga", None, None, &[], None)
            .unwrap();

        let results = index.search("Test", None).unwrap();
        assert_eq!(results.len(), 1);

        index.remove_document(1).unwrap();

        // After removal, re-search
        let reader = index.reader.clone();
        reader.reload().unwrap();
        let results = index.search("Test", None).unwrap();
        assert_eq!(results.len(), 0);
    }
}
