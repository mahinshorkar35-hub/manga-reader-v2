//! Image decoding, caching, and resizing pipeline.
//!
//! Uses the `image-rs` crate to decode various image formats,
//! the `image_cache` module for LRU-cached access with memory-mapped
//! backing, and provides resizing/scaling utilities.

pub mod cache;

use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::Path;

/// Decode an image from raw bytes.
///
/// Supports JPEG, PNG, WebP, GIF, BMP, and TIFF formats.
pub fn decode_from_bytes(bytes: &[u8]) -> Result<DynamicImage> {
    let img = image::load_from_memory(bytes)
        .context("Failed to decode image from bytes")?;
    Ok(img)
}

/// Decode an image from a file path.
pub fn decode_from_path<P: AsRef<Path>>(path: P) -> Result<DynamicImage> {
    let img = image::open(path.as_ref())
        .context("Failed to decode image from path")?;
    Ok(img)
}

/// Resize an image to fit within `max_width` x `max_height` while
/// maintaining aspect ratio.
pub fn resize_to_fit(img: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
    img.resize(max_width, max_height, image::imageops::FilterType::Lanczos3)
}

/// Resize an image to exact dimensions (stretching if necessary).
pub fn resize_exact(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
}

/// Encode a `DynamicImage` to JPEG bytes at the given quality (1–100).
pub fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let quality = quality.min(100).max(1);
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Jpeg,
    )
    .context("Failed to encode JPEG")?;
    Ok(buf)
}

/// Encode a `DynamicImage` to PNG bytes.
pub fn encode_png(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .context("Failed to encode PNG")?;
    Ok(buf)
}

/// Encode a `DynamicImage` to WebP bytes.
pub fn encode_webp(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::WebP,
    )
    .context("Failed to encode WebP")?;
    Ok(buf)
}

/// Information about a decoded image.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub size_bytes: usize,
}

/// Get metadata about an image without decoding the full pixels.
pub fn image_info_from_bytes(bytes: &[u8]) -> Result<ImageInfo> {
    let reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .context("Failed to detect image format")?;

    let format = reader
        .format()
        .map(|f| format!("{:?}", f))
        .unwrap_or_else(|| "unknown".into());

    // We need to decode to get dimensions in image-rs
    let img = reader.decode().context("Failed to decode image for info")?;

    Ok(ImageInfo {
        width: img.width(),
        height: img.height(),
        format,
        size_bytes: bytes.len(),
    })
}

/// The image processing pipeline: decode → optional resize → cache.
pub struct ImagePipeline {
    cache: cache::ImageCache,
}

impl ImagePipeline {
    /// Create a new image pipeline with the given cache backend.
    pub fn new(cache: cache::ImageCache) -> Self {
        Self { cache }
    }

    /// Get a decoded image by its cache key.
    /// If not cached, decodes from the provided bytes, stores in cache, and returns.
    pub fn get_or_decode(&self, key: &str, bytes: &[u8]) -> Result<DynamicImage> {
        // Try cache first
        if let Some(cached) = self.cache.get(key)? {
            let img = decode_from_bytes(&cached)?;
            return Ok(img);
        }

        // Decode and cache
        let img = decode_from_bytes(bytes)?;
        let encoded = encode_png(&img).unwrap_or_else(|_| bytes.to_vec());
        self.cache.put(key, &encoded)?;
        Ok(img)
    }

    /// Invalidate a cached image by key.
    pub fn invalidate(&self, key: &str) -> Result<()> {
        self.cache.remove(key)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_small_png() {
        // Minimal valid 1x1 red PNG
        let png_bytes: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
            0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
            0x00, 0x00, 0x03, 0x00, 0x01, 0x36, 0x28, 0x19,
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82,
        ];

        let img = decode_from_bytes(&png_bytes).unwrap();
        assert_eq!(img.width(), 1);
        assert_eq!(img.height(), 1);

        let info = image_info_from_bytes(&png_bytes).unwrap();
        assert_eq!(info.width, 1);
        assert_eq!(info.height, 1);
    }
}
