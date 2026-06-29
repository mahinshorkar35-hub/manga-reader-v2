namespace MangaReader.Models;

/// <summary>
/// Tracks the user's reading progress for a specific manga.
/// </summary>
public class ReadingProgress
{
    /// <summary>
    /// Manga identifier this progress belongs to.
    /// </summary>
    public string MangaId { get; set; } = string.Empty;

    /// <summary>
    /// Current chapter the user is reading (1-based).
    /// </summary>
    public int CurrentChapter { get; set; } = 1;

    /// <summary>
    /// Last page index viewed in the current chapter.
    /// </summary>
    public int LastPageIndex { get; set; }

    /// <summary>
    /// Total pages in the current chapter.
    /// </summary>
    public int TotalPagesInChapter { get; set; }

    /// <summary>
    /// Percentage of the manga completed overall (0.0–1.0).
    /// </summary>
    public double ProgressPercent { get; set; }

    /// <summary>
    /// Whether the manga has been fully read.
    /// </summary>
    public bool IsComplete { get; set; }

    /// <summary>
    /// Timestamp of the last read action.
    /// </summary>
    public DateTime LastReadAt { get; set; } = DateTime.UtcNow;

    /// <summary>
    /// Reading direction preference for this title.
    /// </summary>
    public ReadingDirection Direction { get; set; } = ReadingDirection.RightToLeft;
}

/// <summary>
/// Reading direction preference.
/// </summary>
public enum ReadingDirection
{
    LeftToRight,
    RightToLeft,
    Vertical
}
