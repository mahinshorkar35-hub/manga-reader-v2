namespace MangaReader.Models;

/// <summary>
/// Represents a single page (image) within a manga chapter.
/// </summary>
public class Page
{
    /// <summary>
    /// Zero-based page index within the chapter.
    /// </summary>
    public int Index { get; set; }

    /// <summary>
    /// Source path or URI for the page image.
    /// The Rust engine provides this via JSON-RPC.
    /// </summary>
    public string ImageSource { get; set; } = string.Empty;

    /// <summary>
    /// Width of the source image in pixels (if known).
    /// </summary>
    public int Width { get; set; }

    /// <summary>
    /// Height of the source image in pixels (if known).
    /// </summary>
    public int Height { get; set; }

    /// <summary>
    /// Whether this page has been fully downloaded / decoded.
    /// </summary>
    public bool IsLoaded { get; set; }

    /// <summary>
    /// Whether this page is a double-page spread.
    /// </summary>
    public bool IsSpread { get; set; }

    /// <summary>
    /// Optional label (e.g., "Cover", "001", "Back Cover").
    /// </summary>
    public string Label { get; set; } = string.Empty;
}
