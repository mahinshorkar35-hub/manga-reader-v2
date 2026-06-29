using SkiaSharp;

namespace MangaReader.Services;

/// <summary>
/// Caches decoded cover and page thumbnails on disk to avoid
/// re-decoding from raw archive data on every navigation.
/// </summary>
public class CacheService : IDisposable
{
    private readonly string _cacheRoot;
    private readonly string _thumbnailsDir;
    private readonly string _coversDir;

    private const string ThumbnailDirName = "thumbnails";
    private const string CoverDirName = "covers";

    /// <summary>
    /// Maximum age for a cached thumbnail before it is considered stale (default: 7 days).
    /// </summary>
    public TimeSpan MaxAge { get; set; } = TimeSpan.FromDays(7);

    public CacheService(string? customCacheRoot = null)
    {
        _cacheRoot = customCacheRoot ??
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "MangaReader", "cache");

        _thumbnailsDir = Path.Combine(_cacheRoot, ThumbnailDirName);
        _coversDir = Path.Combine(_cacheRoot, CoverDirName);

        Directory.CreateDirectory(_thumbnailsDir);
        Directory.CreateDirectory(_coversDir);
    }

    /// <summary>
    /// Get a cached cover image for the given manga ID.
    /// Returns null if not cached or expired.
    /// </summary>
    public SKBitmap? GetCover(string mangaId)
    {
        var path = GetCoverPath(mangaId);
        return LoadFromCache(path);
    }

    /// <summary>
    /// Store a cover image in the cache.
    /// </summary>
    public void StoreCover(string mangaId, SKBitmap bitmap)
    {
        var path = GetCoverPath(mangaId);
        SaveToCache(path, bitmap);
    }

    /// <summary>
    /// Get a cached page thumbnail for the given manga/chapter/page.
    /// </summary>
    public SKBitmap? GetThumbnail(string mangaId, int chapter, int pageIndex)
    {
        var path = GetThumbnailPath(mangaId, chapter, pageIndex);
        return LoadFromCache(path);
    }

    /// <summary>
    /// Store a page thumbnail in the cache.
    /// </summary>
    public void StoreThumbnail(string mangaId, int chapter, int pageIndex, SKBitmap bitmap)
    {
        var path = GetThumbnailPath(mangaId, chapter, pageIndex);
        SaveToCache(path, bitmap);
    }

    /// <summary>
    /// Remove all cached entries for a specific manga.
    /// </summary>
    public void InvalidateManga(string mangaId)
    {
        var cover = GetCoverPath(mangaId);
        if (File.Exists(cover)) File.Delete(cover);

        var mangaThumbDir = Path.Combine(_thumbnailsDir, SanitizeId(mangaId));
        if (Directory.Exists(mangaThumbDir))
            Directory.Delete(mangaThumbDir, recursive: true);
    }

    /// <summary>
    /// Clear the entire cache.
    /// </summary>
    public void ClearAll()
    {
        if (Directory.Exists(_coversDir))
        {
            Directory.Delete(_coversDir, recursive: true);
            Directory.CreateDirectory(_coversDir);
        }

        if (Directory.Exists(_thumbnailsDir))
        {
            Directory.Delete(_thumbnailsDir, recursive: true);
            Directory.CreateDirectory(_thumbnailsDir);
        }
    }

    /// <summary>
    /// Total size of the cache in bytes.
    /// </summary>
    public long GetCacheSizeBytes()
    {
        long size = 0;
        if (Directory.Exists(_cacheRoot))
        {
            foreach (var file in Directory.EnumerateFiles(_cacheRoot, "*", SearchOption.AllDirectories))
            {
                try { size += new FileInfo(file).Length; } catch { /* skip locked files */ }
            }
        }
        return size;
    }

    // ── private helpers ──────────────────────────────────────────────────

    private string GetCoverPath(string mangaId) =>
        Path.Combine(_coversDir, $"{SanitizeId(mangaId)}.png");

    private string GetThumbnailPath(string mangaId, int chapter, int pageIndex)
    {
        var dir = Path.Combine(_thumbnailsDir, SanitizeId(mangaId), $"ch{chapter}");
        Directory.CreateDirectory(dir);
        return Path.Combine(dir, $"p{pageIndex:D4}.png");
    }

    private static string SanitizeId(string id)
    {
        var invalid = Path.GetInvalidFileNameChars();
        return string.Join("_", id.Split(invalid, StringSplitOptions.RemoveEmptyEntries));
    }

    private SKBitmap? LoadFromCache(string path)
    {
        if (!File.Exists(path))
            return null;

        var lastWrite = File.GetLastWriteTimeUtc(path);
        if (DateTime.UtcNow - lastWrite > MaxAge)
        {
            File.Delete(path);
            return null;
        }

        try
        {
            using var stream = File.OpenRead(path);
            return SKBitmap.Decode(stream);
        }
        catch
        {
            return null;
        }
    }

    private static void SaveToCache(string path, SKBitmap bitmap)
    {
        try
        {
            using var image = SKImage.FromBitmap(bitmap);
            using var data = image.Encode(SKEncodedImageFormat.Png, 85);
            File.WriteAllBytes(path, data.ToArray());
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"Cache write failed: {ex.Message}");
        }
    }

    public void Dispose()
    {
        // Nothing to dispose — Bitmaps are caller-owned.
        GC.SuppressFinalize(this);
    }
}
