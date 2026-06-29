//! Gutter-based panel splitting.
//!
//! Ported from the Python algorithm in the original manga-panel-extractor.
//!
//! ## Algorithm
//! 1. Convert the page to grayscale
//! 2. Apply thresholding to create a binary image (pixels below threshold = black/ink)
//! 3. Compute horizontal projection profile (sum of dark pixels per row)
//! 4. Detect horizontal gutters: runs of rows where dark pixel count is below a threshold
//! 5. Split the page into horizontal strips at gutter midpoints
//! 6. For each strip, compute vertical projection profile
//! 7. Detect vertical gutters similarly
//! 8. Split each strip into individual panels at vertical gutters
//! 9. Filter out panels that are too small (noise)

use anyhow::Result;
use image::{DynamicImage, GrayImage, Luma};
use std::time::Instant;

use super::{DetectionResult, Panel, PanelDetector};

/// Configuration for the gutter detector.
#[derive(Debug, Clone)]
pub struct GutterConfig {
    /// Binary threshold (0–255). Pixels darker than this are considered "ink".
    pub threshold: u8,
    /// Minimum gutter height/width as a fraction of page dimension.
    /// E.g., 0.02 means a gutter must be at least 2% of the page.
    pub min_gutter_ratio: f64,
    /// Minimum panel dimension as a fraction of page dimension.
    /// Panels smaller than this are discarded as noise.
    pub min_panel_ratio: f64,
    /// Maximum allowed gap between ink pixels to still be considered part
    /// of a gutter (for handling anti-aliased edges).
    pub gutter_continuity_gap: u32,
}

impl Default for GutterConfig {
    fn default() -> Self {
        Self {
            threshold: 200,
            min_gutter_ratio: 0.015,
            min_panel_ratio: 0.03,
            gutter_continuity_gap: 3,
        }
    }
}

/// Gutter-based panel detector.
///
/// Detects panels by finding whitespace gutters between them using
/// projection profiles.
pub struct GutterDetector {
    config: GutterConfig,
}

impl GutterDetector {
    pub fn new() -> Self {
        Self {
            config: GutterConfig::default(),
        }
    }

    pub fn with_config(config: GutterConfig) -> Self {
        Self { config }
    }

    /// Convert a DynamicImage to grayscale.
    fn to_grayscale(img: &DynamicImage) -> GrayImage {
        img.to_luma8()
    }

    /// Apply threshold to produce a binary image where 0 = ink, 255 = white.
    fn threshold_image(img: &GrayImage, threshold: u8) -> GrayImage {
        let mut binary = GrayImage::new(img.width(), img.height());
        for (x, y, pixel) in img.enumerate_pixels() {
            let val = if pixel.0[0] < threshold { 0u8 } else { 255u8 };
            binary.put_pixel(x, y, Luma([val]));
        }
        binary
    }

    /// Compute horizontal projection profile: for each row, count dark pixels.
    fn horizontal_projection(img: &GrayImage) -> Vec<u32> {
        let height = img.height() as usize;
        let width = img.width() as usize;
        let mut proj = vec![0u32; height];

        for y in 0..height {
            let mut count = 0u32;
            for x in 0..width {
                if img.get_pixel(x as u32, y as u32).0[0] == 0 {
                    count += 1;
                }
            }
            proj[y] = count;
        }
        proj
    }

    /// Compute vertical projection profile: for each column, count dark pixels.
    fn vertical_projection(img: &GrayImage) -> Vec<u32> {
        let width = img.width() as usize;
        let height = img.height() as usize;
        let mut proj = vec![0u32; width];

        for x in 0..width {
            let mut count = 0u32;
            for y in 0..height {
                if img.get_pixel(x as u32, y as u32).0[0] == 0 {
                    count += 1;
                }
            }
            proj[x] = count;
        }
        proj
    }

    /// Detect gaps (gutters) in a projection profile.
    ///
    /// A gap is a contiguous range of indices where the projection value
    /// is below the threshold (indicating whitespace).
    fn detect_gaps(
        proj: &[u32],
        threshold: u32,
        min_gap_size: usize,
        continuity_gap: usize,
    ) -> Vec<(usize, usize)> {
        let mut gaps = Vec::new();
        let mut in_gap = false;
        let mut gap_start = 0usize;
        let mut gap_end = 0usize;

        for (i, &val) in proj.iter().enumerate() {
            if val < threshold {
                if !in_gap {
                    in_gap = true;
                    gap_start = i;
                }
                gap_end = i;
            } else {
                if in_gap {
                    // Check if the gap is large enough
                    let gap_size = gap_end - gap_start + 1;
                    if gap_size >= min_gap_size {
                        gaps.push((gap_start, gap_end));
                    }
                    in_gap = false;
                }
            }
        }

        // Handle trailing gap
        if in_gap {
            let gap_size = gap_end - gap_start + 1;
            if gap_size >= min_gap_size {
                gaps.push((gap_start, gap_end));
            }
        }

        // Merge nearby gaps (separated by small "islands" ≤ continuity_gap)
        if !gaps.is_empty() {
            let mut merged = Vec::new();
            let mut current = gaps[0];

            for &next in &gaps[1..] {
                if next.0 <= current.1 + continuity_gap {
                    current.1 = next.1;
                } else {
                    merged.push(current);
                    current = next;
                }
            }
            merged.push(current);
            merged
        } else {
            gaps
        }
    }

    /// Find the midpoints between gutters, which become panel boundaries.
    fn midpoints(gaps: &[(usize, usize)], total_len: usize) -> Vec<usize> {
        let mut split_points = Vec::new();

        // Add boundary at start
        split_points.push(0);

        // Midpoints between consecutive gaps define panel boundaries
        for i in 0..gaps.len() {
            // The midpoint of this gap
            let mid = (gaps[i].0 + gaps[i].1) / 2;
            if mid > 0 && mid < total_len {
                split_points.push(mid);
            }
        }

        // Add boundary at end
        if total_len > 0 {
            split_points.push(total_len - 1);
        }

        split_points.sort();
        split_points.dedup();
        split_points
    }

    /// Extract panel regions from the page using horizontal and vertical splits.
    fn extract_panels(
        _img: &GrayImage,
        h_splits: &[usize],
        v_splits: &[usize],
        min_width: u32,
        min_height: u32,
    ) -> Vec<Panel> {
        let mut panels = Vec::new();
        let mut id = 0usize;

        for h_idx in 0..h_splits.len() - 1 {
            let y0 = h_splits[h_idx] as u32;
            let y1 = h_splits[h_idx + 1] as u32;

            for v_idx in 0..v_splits.len() - 1 {
                let x0 = v_splits[v_idx] as u32;
                let x1 = v_splits[v_idx + 1] as u32;

                let width = x1.saturating_sub(x0);
                let height = y1.saturating_sub(y0);

                // Filter out noise: panels must meet minimum size
                if width >= min_width && height >= min_height {
                    panels.push(Panel::new(id, x0, y0, width, height));
                    id += 1;
                }
            }
        }

        panels
    }
}

impl PanelDetector for GutterDetector {
    fn detect(&self, page: &DynamicImage) -> Result<DetectionResult> {
        let start = Instant::now();
        let page_width = page.width();
        let page_height = page.height();

        log::info!(
            "GutterDetector: detecting panels on {}x{} page",
            page_width,
            page_height
        );

        // Step 1: Convert to grayscale
        let gray = Self::to_grayscale(page);

        // Step 2: Threshold to binary (0 = ink, 255 = white)
        let binary = Self::threshold_image(&gray, self.config.threshold);

        // Step 3: Compute horizontal projection
        let h_proj = Self::horizontal_projection(&binary);
        let max_dark = page_width as u32; // Max possible dark pixels in a row

        // Threshold for gutter detection: rows with < 5% ink in them
        let h_gutter_threshold = (max_dark as f64 * 0.05) as u32;
        let min_h_gap = (page_height as f64 * self.config.min_gutter_ratio) as usize;
        let con_gap = self.config.gutter_continuity_gap as usize;

        // Step 4: Detect horizontal gutters
        let h_gaps = Self::detect_gaps(&h_proj, h_gutter_threshold, min_h_gap.max(1), con_gap);

        log::info!("  Horizontal gaps: {:?}", h_gaps);

        // Step 5: Compute horizontal split points
        let h_splits = Self::midpoints(&h_gaps, page_height as usize);

        log::info!("  Horizontal splits: {:?}", h_splits);

        let min_panel_w = (page_width as f64 * self.config.min_panel_ratio) as u32;
        let min_panel_h = (page_height as f64 * self.config.min_panel_ratio) as u32;

        let mut all_panels = Vec::new();

        // Step 6: For each horizontal strip, compute vertical projection
        for i in 0..h_splits.len() - 1 {
            let y_start = h_splits[i] as u32;
            let y_end = (h_splits[i + 1] as u32).min(page_height);

            if y_end <= y_start {
                continue;
            }

            let strip_height = y_end - y_start;
            if strip_height < min_panel_h {
                continue;
            }

            // Crop the strip for vertical analysis
            let strip = image::imageops::crop_imm(&binary, 0, y_start, page_width, strip_height).to_image();
            let v_proj = Self::vertical_projection(&strip);

            let v_gutter_threshold = (strip_height as f64 * 0.05) as u32;
            let min_v_gap = (page_width as f64 * self.config.min_gutter_ratio) as usize;

            let v_gaps = Self::detect_gaps(&v_proj, v_gutter_threshold, min_v_gap.max(1), con_gap);

            log::info!("  Strip {}: vertical gaps: {:?}", i, v_gaps);

            let v_splits = Self::midpoints(&v_gaps, page_width as usize);

            log::info!("  Strip {}: vertical splits: {:?}", i, v_splits);

            // Extract panels from this strip
            let panels = Self::extract_panels(
                &binary,
                &[h_splits[i], h_splits[i + 1]],
                &v_splits,
                min_panel_w,
                min_panel_h,
            );

            all_panels.extend(panels);
        }

        // Re-number panels sequentially
        for (idx, panel) in all_panels.iter_mut().enumerate() {
            panel.id = idx;
        }

        let elapsed = start.elapsed();
        let ms = elapsed.as_millis() as u64;

        log::info!("GutterDetector: found {} panels in {} ms", all_panels.len(), ms);

        Ok(DetectionResult {
            panels: all_panels,
            page_width,
            page_height,
            detector: "gutter".into(),
            processing_time_ms: ms,
        })
    }

    fn name(&self) -> &str {
        "gutter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_detection() {
        let proj = vec![
            100u32, 100, 100, // top ink
            0, 0, 0, // gap
            100, 100, 100, // middle ink
            0, 0, 0, 0, // gap
            100, 100, // bottom ink
        ];

        let gaps = GutterDetector::detect_gaps(&proj, 10, 2, 1);
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0], (3, 5));
        assert_eq!(gaps[1], (9, 12));
    }

    #[test]
    fn test_midpoints() {
        let gaps = vec![(3usize, 5usize), (9usize, 12usize)];
        let mids = GutterDetector::midpoints(&gaps, 15);
        assert_eq!(mids, vec![0, 4, 10, 14]);
    }

    #[test]
    fn test_empty_page() {
        let page = DynamicImage::new_luma8(100, 100);
        let detector = GutterDetector::new();
        let result = detector.detect(&page).unwrap();
        // All-white page should produce either 0 or 1 panel
        assert!(result.panels.is_empty() || result.panels.len() <= 1);
    }
}
