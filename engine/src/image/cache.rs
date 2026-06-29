//! LRU image cache with memory-mapped file backing.
//!
//! Provides a disk-backed LRU cache for decoded (or encoded) image
//! data. The cache uses:
//! - An `lru::LruCache` for in-memory hot entries
//! - Memory-mapped files (via `memmap2`) for cold entries on disk
//! - A JSON index file to track cached entries and eviction ordering

use anyhow::{Context, Result};
use lru::LruCache;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A single entry in the cache index.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheIndexEntry {
    /// Relative path to the cached file (within cache_dir).
    file_path: String,
    /// Size of the cached data in bytes.
    size: u64,
    /// Monotonically increasing access counter for LRU ordering.
    access_count: u64,
}

/// The on-disk index that tracks all cached items.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheIndex {
    entries: HashMap<String, CacheIndexEntry>,
    version: u32,
}

impl CacheIndex {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            version: 1,
        }
    }
}

/// Stale file cleanup policy.
#[derive(Debug, Clone, Copy)]
pub enum EvictionPolicy {
    /// Evict least-recently-used items when cache exceeds max size.
    Lru,
    /// Evict largest items first when cache exceeds max size.
    LargestFirst,
}

/// A disk-backed LRU image cache with memory-mapped file access.
///
/// ## Design
/// - **Hot cache**: In-memory `LruCache` for frequently accessed entries.
/// - **Cold cache**: Files on disk, accessed via `memmap2` for zero-copy reads.
/// - **Index**: A `cache_index.json` file in the cache directory tracks all entries.
///
/// When total disk usage exceeds `max_size`, the eviction policy removes
/// entries until usage drops below `max_size * 0.9`.
pub struct ImageCache {
    /// Directory where cached files are stored.
    cache_dir: PathBuf,
    /// Maximum total cache size in bytes.
    max_size: u64,
    /// In-memory LRU cache for hot entries.
    hot_cache: Mutex<LruCache<String, Vec<u8>>>,
    /// The on-disk index (guarded by mutex).
    index: Mutex<CacheIndex>,
    /// Eviction policy.
    policy: EvictionPolicy,
    /// Monotonically increasing access counter.
    access_counter: Mutex<u64>,
}

/// Manual `Clone` implementation because `Mutex` does not implement `Clone` in std.
impl Clone for ImageCache {
    fn clone(&self) -> Self {
        Self {
            cache_dir: self.cache_dir.clone(),
            max_size: self.max_size,
            hot_cache: Mutex::new(self.hot_cache.lock().unwrap().clone()),
            index: Mutex::new(self.index.lock().unwrap().clone()),
            policy: self.policy,
            access_counter: Mutex::new(*self.access_counter.lock().unwrap()),
        }
    }
}

impl ImageCache {
    /// Create a new image cache rooted at `cache_dir` with the given max size.
    ///
    /// If `cache_dir` doesn't exist, it will be created. An existing cache
    /// index will be loaded if present.
    pub fn new<P: AsRef<Path>>(cache_dir: P, max_size: u64) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        // Load or create the cache index
        let index_path = cache_dir.join("cache_index.json");
        let index = if index_path.exists() {
            let content = fs::read_to_string(&index_path)
                .context("Failed to read cache index")?;
            serde_json::from_str(&content).unwrap_or_else(|_| CacheIndex::new())
        } else {
            CacheIndex::new()
        };

        // Calculate current disk usage
        let current_size: u64 = index.entries.values().map(|e| e.size).sum();

        log::info!(
            "ImageCache initialized: dir={}, max_size={}, current_usage={}, entries={}",
            cache_dir.display(),
            max_size,
            current_size,
            index.entries.len()
        );

        let hot_capacity = NonZeroUsize::new(100).unwrap(); // Keep 100 hot entries in memory
        let hot_cache = LruCache::new(hot_capacity);

        Ok(Self {
            cache_dir,
            max_size,
            hot_cache: Mutex::new(hot_cache),
            index: Mutex::new(index),
            policy: EvictionPolicy::Lru,
            access_counter: Mutex::new(0),
        })
    }

    /// Retrieve cached data for the given key.
    ///
    /// Checks the hot cache first, then falls back to the memory-mapped
    /// file on disk. Returns `None` if the key is not cached.
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Check hot cache first
        {
            let mut hot = self.hot_cache.lock().unwrap();
            if let Some(data) = hot.get(key) {
                return Ok(Some(data.clone()));
            }
        }

        // Check disk cache via index
        {
            let mut index = self.index.lock().unwrap();
            if let Some(entry) = index.entries.get(key) {
                let file_path = self.cache_dir.join(&entry.file_path);
                if file_path.exists() {
                    // Memory-map the file for zero-copy access
                    let file = fs::File::open(&file_path)
                        .context("Failed to open cached file")?;
                    let mmap = unsafe { Mmap::map(&file) }
                        .context("Failed to memory-map cached file")?;
                    let data = mmap[..].to_vec();

                    // Update access count
                    let mut counter = self.access_counter.lock().unwrap();
                    *counter += 1;
                    let index_entry = index.entries.get_mut(key).unwrap();
                    index_entry.access_count = *counter;

                    // Promote to hot cache
                    let mut hot = self.hot_cache.lock().unwrap();
                    hot.put(key.to_string(), data.clone());

                    return Ok(Some(data));
                } else {
                    // File missing from disk — remove from index
                    index.entries.remove(key);
                    self.save_index()?;
                }
            }
        }

        Ok(None)
    }

    /// Insert data into the cache under the given key.
    ///
    /// The data is written to a file on disk and added to the index.
    /// If the cache exceeds `max_size`, eviction is triggered.
    pub fn put(&self, key: &str, data: &[u8]) -> Result<()> {
        // Sanitize the key for use as a filename
        let file_name = sanitize_filename(key);
        let file_path = self.cache_dir.join(&file_name);

        // Write data to disk
        fs::write(&file_path, data)
            .context(format!("Failed to write cache file: {}", file_path.display()))?;

        let size = data.len() as u64;

        // Update index
        {
            let mut index = self.index.lock().unwrap();
            let mut counter = self.access_counter.lock().unwrap();
            *counter += 1;

            index.entries.insert(key.to_string(), CacheIndexEntry {
                file_path: file_name,
                size,
                access_count: *counter,
            });
        }

        // Update hot cache
        {
            let mut hot = self.hot_cache.lock().unwrap();
            hot.put(key.to_string(), data.to_vec());
        }

        // Evict if over limit
        self.enforce_size_limit()?;

        // Persist index
        self.save_index()?;

        Ok(())
    }

    /// Remove a specific entry from the cache.
    pub fn remove(&self, key: &str) -> Result<bool> {
        let mut index = self.index.lock().unwrap();
        if let Some(entry) = index.entries.remove(key) {
            let file_path = self.cache_dir.join(&entry.file_path);
            if file_path.exists() {
                fs::remove_file(&file_path).ok();
            }

            let mut hot = self.hot_cache.lock().unwrap();
            hot.pop(key);

            self.save_index()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Clear the entire cache (both hot and disk).
    pub fn clear(&self) -> Result<()> {
        // Clear hot cache
        let mut hot = self.hot_cache.lock().unwrap();
        hot.clear();

        // Remove all disk files
        let mut index = self.index.lock().unwrap();
        for entry in index.entries.values() {
            let file_path = self.cache_dir.join(&entry.file_path);
            if file_path.exists() {
                fs::remove_file(&file_path).ok();
            }
        }
        index.entries.clear();

        self.save_index()?;

        // Remove index file
        let index_path = self.cache_dir.join("cache_index.json");
        if index_path.exists() {
            fs::remove_file(&index_path).ok();
        }

        Ok(())
    }

    /// Get the current total size of cached data.
    pub fn total_size(&self) -> u64 {
        let index = self.index.lock().unwrap();
        index.entries.values().map(|e| e.size).sum()
    }

    /// Get the number of cached entries.
    pub fn entry_count(&self) -> usize {
        let index = self.index.lock().unwrap();
        index.entries.len()
    }

    /// Enforce the max size limit by evicting entries.
    fn enforce_size_limit(&self) -> Result<()> {
        let mut total_size = self.total_size();
        if total_size <= self.max_size {
            return Ok(());
        }

        log::info!(
            "Cache size {} exceeds limit {}, evicting...",
            total_size,
            self.max_size
        );

        let target_size = (self.max_size as f64 * 0.9) as u64;

        loop {
            if total_size <= target_size {
                break;
            }

            let evict_key = {
                let index = self.index.lock().unwrap();
                match self.policy {
                    EvictionPolicy::Lru => {
                        index.entries.iter()
                            .min_by_key(|(_, e)| e.access_count)
                            .map(|(k, _)| k.clone())
                    }
                    EvictionPolicy::LargestFirst => {
                        index.entries.iter()
                            .max_by_key(|(_, e)| e.size)
                            .map(|(k, _)| k.clone())
                    }
                }
            };

            if let Some(key) = evict_key {
                if let Some(entry) = {
                    let mut index = self.index.lock().unwrap();
                    index.entries.remove(&key)
                } {
                    let file_path = self.cache_dir.join(&entry.file_path);
                    if file_path.exists() {
                        fs::remove_file(&file_path).ok();
                    }
                    let mut hot = self.hot_cache.lock().unwrap();
                    hot.pop(&key);
                    total_size -= entry.size;
                }
            } else {
                break;
            }
        }

        self.save_index()?;
        Ok(())
    }

    /// Persist the cache index to disk.
    fn save_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("cache_index.json");
        let index = self.index.lock().unwrap();
        let content = serde_json::to_string_pretty(&*index)
            .context("Failed to serialize cache index")?;
        fs::write(&index_path, content)
            .context("Failed to write cache index")?;
        Ok(())
    }
}

impl Drop for ImageCache {
    fn drop(&mut self) {
        // Save index on drop
        self.save_index().ok();
    }
}

/// Sanitize a string to be a safe filename.
fn sanitize_filename(key: &str) -> String {
    let sanitized: String = key
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            '/' | '\\' => '_',
            _ => format!("{:02x}", c as u8).chars().next().unwrap_or('_'),
        })
        .collect();

    // Ensure it doesn't start with a dot (hidden files)
    let sanitized = if sanitized.starts_with('.') {
        format!("_{}", sanitized)
    } else {
        sanitized
    };

    // Truncate excessively long names (Windows MAX_PATH issues)
    if sanitized.len() > 200 {
        let hash = blake2_simple(&sanitized);
        format!("{}_{}", &sanitized[..180], hash)
    } else {
        sanitized
    }
}

/// Simple hash for filename truncation.
fn blake2_simple(input: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_put_get() {
        let temp_dir = std::env::temp_dir().join("manga_cache_test");
        let _ = fs::remove_dir_all(&temp_dir);

        let cache = ImageCache::new(&temp_dir, 1024 * 1024).unwrap();

        cache.put("page_001", b"test_image_data_123").unwrap();
        let result = cache.get("page_001").unwrap();
        assert_eq!(result, Some(b"test_image_data_123".to_vec()));

        cache.clear().unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_miss() {
        let temp_dir = std::env::temp_dir().join("manga_cache_test_miss");
        let _ = fs::remove_dir_all(&temp_dir);

        let cache = ImageCache::new(&temp_dir, 1024 * 1024).unwrap();
        let result = cache.get("nonexistent").unwrap();
        assert_eq!(result, None);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("simple_key"), "simple_key");
        assert_eq!(sanitize_filename("path/to/file"), "path_to_file");
        assert!(!sanitize_filename(".hidden").starts_with('.'));
    }
}
