using SkiaSharp;
using SkiaSharp.Views.Windows;
using MangaReader.Models;

namespace MangaReader.Controls;

/// <summary>
/// Custom control that renders a manga cover image with an
/// unread-count badge overlay using SkiaSharp.
/// </summary>
public class MangaCover : SKXamlCanvas
{
    private SKBitmap? _coverBitmap;

    /// <summary>
    /// The manga model used for badge information.
    /// </summary>
    public Manga? Manga { get; set; }

    /// <summary>
    /// The cover bitmap to display.
    /// </summary>
    public SKBitmap? CoverBitmap
    {
        get => _coverBitmap;
        set
        {
            _coverBitmap = value;
            Invalidate();
        }
    }

    /// <summary>
    /// Corner radius for the cover image.
    /// </summary>
    public float CornerRadius { get; set; } = 8f;

    /// <summary>
    /// Whether to show the unread badge.
    /// </summary>
    public bool ShowBadge { get; set; } = true;

    public MangaCover()
    {
        EnableRenderLoop = false;
    }

    protected override void OnPaintSurface(SKPaintGLSurfaceEventArgs e)
    {
        var canvas = e.Surface.Canvas;
        var info = e.Info;

        // Background
        canvas.Clear(SKColor.Parse("#2D2D2D"));

        if (_coverBitmap == null)
        {
            DrawPlaceholderCover(canvas, info);
            return;
        }

        // Calculate cover area (maintain aspect ratio, fill width)
        float scale = (float)info.Width / _coverBitmap.Width;
        float drawHeight = _coverBitmap.Height * scale;
        float yOffset = (info.Height - drawHeight) / 2f;

        var destRect = new SKRect(0, Math.Max(0, yOffset), info.Width, Math.Min(info.Height, yOffset + drawHeight));

        // Clip to rounded rect
        using var clipPath = new SKPath();
        clipPath.AddRoundRect(new SKRect(0, 0, info.Width, info.Height), CornerRadius, CornerRadius);
        canvas.ClipPath(clipPath, SKClipOperation.Intersect, true);

        // Draw cover image
        canvas.DrawBitmap(_coverBitmap, destRect);

        // Bottom gradient overlay for badge readability
        if (ShowBadge && Manga?.UnreadChapters > 0)
        {
            using var gradientPaint = new SKPaint
            {
                Shader = SKShader.CreateLinearGradient(
                    new SKPoint(0, info.Height - 50),
                    new SKPoint(0, info.Height),
                    new[] { SKColors.Transparent, SKColor.Parse("#CC000000") },
                    SKShaderTileMode.Clamp)
            };
            canvas.DrawRect(0, info.Height - 50, info.Width, 50, gradientPaint);

            // Draw unread badge
            DrawBadge(canvas, info, Manga.UnreadChapters);
        }

        // Title bar at bottom
        DrawTitleBar(canvas, info);
    }

    private void DrawPlaceholderCover(SKCanvas canvas, SKImageInfo info)
    {
        // Draw a placeholder icon
        using var paint = new SKPaint
        {
            Color = SKColor.Parse("#444444"),
            IsAntialias = true,
            TextAlign = SKTextAlign.Center,
            TextSize = 48
        };

        // Book icon placeholder (unicode character)
        canvas.DrawText("\uD83D\uDCDA", info.Width / 2f, info.Height / 2f + 16, paint);

        if (Manga != null)
        {
            paint.TextSize = 14;
            paint.Color = SKColor.Parse("#999999");
            canvas.DrawText(Manga.Title, info.Width / 2f, info.Height - 16, paint);
        }
    }

    private void DrawBadge(SKCanvas canvas, SKImageInfo info, int unread)
    {
        var badgeText = unread > 99 ? "99+" : unread.ToString();

        // Badge circle size
        float badgeSize = 28f;
        float margin = 8f;
        float cx = info.Width - margin - badgeSize / 2f;
        float cy = info.Height - margin - badgeSize / 2f;

        // Shadow
        using var shadowPaint = new SKPaint
        {
            Color = SKColor.Parse("#80000000"),
            IsAntialias = true,
            MaskFilter = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, 3)
        };
        canvas.DrawCircle(cx + 1, cy + 1, badgeSize / 2f, shadowPaint);

        // Red badge circle
        using var badgePaint = new SKPaint
        {
            Color = SKColor.Parse("#FF4444"),
            IsAntialias = true
        };
        canvas.DrawCircle(cx, cy, badgeSize / 2f, badgePaint);

        // Badge text
        using var textPaint = new SKPaint
        {
            Color = SKColors.White,
            IsAntialias = true,
            TextAlign = SKTextAlign.Center,
            TextSize = 13
        };
        canvas.DrawText(badgeText, cx, cy + 5, textPaint);
    }

    private void DrawTitleBar(SKCanvas canvas, SKImageInfo info)
    {
        if (Manga == null) return;

        // Semi-transparent title bar
        using var titleBgPaint = new SKPaint
        {
            Color = SKColor.Parse("#AA000000"),
        };
        canvas.DrawRect(0, 0, info.Width, 36, titleBgPaint);

        // Title text
        using var titlePaint = new SKPaint
        {
            Color = SKColors.White,
            IsAntialias = true,
            TextSize = 14
        };

        var title = Manga.Title;
        if (titlePaint.MeasureText(title) > info.Width - 16)
        {
            // Truncate with ellipsis
            while (titlePaint.MeasureText(title + "…") > info.Width - 16 && title.Length > 1)
                title = title[..^1];
            title += "…";
        }

        canvas.DrawText(title, 8, 24, titlePaint);
    }

    /// <summary>
    /// Release the cover bitmap resources.
    /// </summary>
    public void ReleaseCover()
    {
        _coverBitmap?.Dispose();
        _coverBitmap = null;
        Invalidate();
    }
}
