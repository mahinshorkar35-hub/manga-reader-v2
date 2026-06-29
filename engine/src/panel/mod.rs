//! Panel detection interface and implementations.
//!
//! Provides abstractions for detecting manga/comic panels within
//! a page image. Currently supports:
//! - **Gutter-based splitting**: Detects whitespace gutters between panels
//!   and splits the page along them (ported from the Python algorithm).
//!
//! Future implementations may include ML-based detection using YOLO.

pub mod gutter;

use anyhow::Result;
use image::DynamicImage;

/// A single detected panel region within a page.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Panel {
    /// Unique identifier for this panel within the page.
    pub id: usize,
    /// X coordinate of the top-left corner (pixels).
    pub x: u32,
    /// Y coordinate of the top-left corner (pixels).
    pub y: u32,
    /// Width of the panel (pixels).
    pub width: u32,
    /// Height of the panel (pixels).
    pub height: u32,
    /// Confidence score (0.0–1.0) for ML-based detection.
    /// For gutter-based detection, this is always 1.0.
    pub confidence: f64,
    /// The extracted panel image (if extracted).
    #[serde(skip)]
    pub image: Option<DynamicImage>,
}

impl Panel {
    /// Create a new panel from bounding box coordinates.
    pub fn new(id: usize, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            confidence: 1.0,
            image: None,
        }
    }

    /// Extract the panel image from the full page image.
    pub fn extract_from(&self, page: &DynamicImage) -> Option<DynamicImage> {
        if self.x + self.width > page.width() || self.y + self.height > page.height() {
            return None;
        }
        Some(page.crop_imm(self.x, self.y, self.width, self.height))
    }
}

/// Detection result for a single page.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectionResult {
    /// The panels detected on this page.
    pub panels: Vec<Panel>,
    /// Page dimensions (width, height).
    pub page_width: u32,
    pub page_height: u32,
    /// Which detector was used.
    pub detector: String,
    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
}

/// Trait for panel detection algorithms.
///
/// Implementations can range from simple heuristic-based
/// gutter detection to ML-based YOLO inference.
pub trait PanelDetector: Send + Sync {
    /// Detect panels in a full-page image.
    fn detect(&self, page: &DynamicImage) -> Result<DetectionResult>;

    /// Name of this detector (for logging/metrics).
    fn name(&self) -> &str;
}

/// Create the default panel detector (gutter-based).
pub fn default_detector() -> Box<dyn PanelDetector> {
    Box::new(gutter::GutterDetector::new())
}
