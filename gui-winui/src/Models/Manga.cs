namespace MangaReader.Models;

/// <summary>
/// Represents a single manga series in the library.
/// </summary>
public class Manga
{
    /// <summary>
    /// Unique identifier for this manga (matches the Rust engine's ID).
    /// </summary>
    public string Id { get; set; } = string.Empty;

    /// <summary>
    /// Display title of the manga.
    /// </summary>
    public string Title { get; set; } = string.Empty;

    /// <summary>
    /// Author or creator of the manga.
    /// </summary>
    public string Author { get; set; } = string.Empty;

    /// <summary>
    /// Short description or synopsis.
    /// </summary>
    public string Description { get; set; } = string.Empty;

    /// <summary>
    /// URL or local path to the cover image.
    /// </summary>
    public string CoverPath { get; set; } = string.Empty;

    /// <summary>
    /// Total number of chapters available.
    /// </summary>
    public int TotalChapters { get; set; }

    /// <summary>
    /// Number of unread chapters.
    /// </summary>
    public int UnreadChapters { get; set; }

    /// <summary>
    /// Comma-separated list of tags/genres.
    /// </summary>
    public string Tags { get; set; } = string.Empty;

    /// <summary>
    /// File format of the source archive (e.g., "cbz", "cbr", "pdf").
    /// </summary>
    public string Format { get; set; } = "cbz";

    /// <summary>
    /// Date the manga was added to the library.
    /// </summary>
    public DateTime AddedDate { get; set; } = DateTime.UtcNow;

    /// <summary>
    /// Date of the last read session.
    /// </summary>
    public DateTime? LastReadDate { get; set; }

    /// <summary>
    /// Whether this manga is marked as a favorite.
    /// </summary>
    public bool IsFavorite { get; set; }
}
