using SkiaSharp;
using SkiaSharp.Views.Windows;

namespace MangaReader.Controls;

/// <summary>
/// GPU-accelerated page renderer using SkiaSharp.
/// Renders manga pages at native resolution with pan/zoom support.
/// </summary>
public class PageView : SKXamlCanvas
{
    private SKBitmap? _bitmap;
    private SKPoint _offset = SKPoint.Empty;
    private float _scale = 1f;
    private SKPoint _lastPan;
    private bool _isPanning;

    /// <summary>
    /// The current page image bitmap to display.
    /// </summary>
    public SKBitmap? Bitmap
    {
        get => _bitmap;
        set
        {
            _bitmap = value;
            _scale = 1f;
            _offset = SKPoint.Empty;
            Invalidate();
        }
    }

    /// <summary>
    /// Current zoom scale factor.
    /// </summary>
    public float ZoomScale => _scale;

    /// <summary>
    /// Whether to fit the page to the control width (default: true).
    /// </summary>
    public bool FitToWidth { get; set; } = true;

    /// <summary>
    /// Whether to fit the page to the control height.
    /// Overrides FitToWidth when the aspect ratio is taller.
    /// </summary>
    public bool FitToHeight { get; set; }

    /// <summary>
    /// Background color for letterboxing areas.
    /// </summary>
    public SKColor BackgroundColor { get; set; } = SKColor.Parse("#1A1A1A");

    public PageView()
    {
        EnableRenderLoop = false;
    }

    protected override void OnPaintSurface(SKPaintGLSurfaceEventArgs e)
    {
        var canvas = e.Surface.Canvas;
        var info = e.Info;

        // Clear background
        canvas.Clear(BackgroundColor);

        if (_bitmap == null)
        {
            DrawPlaceholder(canvas, info);
            return;
        }

        canvas.Save();

        // Apply pan offset
        canvas.Translate(_offset.X, _offset.Y);

        // Calculate scale to fit
        if (FitToWidth || FitToHeight)
        {
            float scaleX = (float)info.Width / _bitmap.Width;
            float scaleY = (float)info.Height / _bitmap.Height;

            if (FitToWidth && FitToHeight)
                _scale = Math.Min(scaleX, scaleY);
            else if (FitToWidth)
                _scale = scaleX;
            else
                _scale = scaleY;
        }

        canvas.Scale(_scale);

        // Center the image
        float centeredX = (info.Width / _scale - _bitmap.Width) / 2f;
        float centeredY = (info.Height / _scale - _bitmap.Height) / 2f;
        canvas.DrawBitmap(_bitmap, centeredX, centeredY);

        canvas.Restore();

        // Draw page info overlay
        DrawOverlay(canvas, info);
    }

    private void DrawPlaceholder(SKCanvas canvas, SKImageInfo info)
    {
        using var paint = new SKPaint
        {
            Color = SKColor.Parse("#333333"),
            IsAntialias = true,
            TextAlign = SKTextAlign.Center,
            TextSize = 24
        };

        canvas.DrawText("No page loaded",
            info.Width / 2f, info.Height / 2f, paint);
    }

    private void DrawOverlay(SKCanvas canvas, SKImageInfo info)
    {
        using var paint = new SKPaint
        {
            Color = SKColors.White.WithAlpha(180),
            IsAntialias = true,
            TextSize = 14
        };

        var zoomText = $"Zoom: {_scale * 100:F0}%";
        canvas.DrawText(zoomText, info.Width - 10, info.Height - 10, paint);
    }

    /// <summary>
    /// Zoom in by a factor.
    /// </summary>
    public void ZoomIn(float factor = 1.25f)
    {
        FitToWidth = false;
        FitToHeight = false;
        _scale *= factor;
        Invalidate();
    }

    /// <summary>
    /// Zoom out by a factor.
    /// </summary>
    public void ZoomOut(float factor = 0.8f)
    {
        _scale = Math.Max(0.1f, _scale * factor);
        Invalidate();
    }

    /// <summary>
    /// Reset zoom to fit the control.
    /// </summary>
    public void ResetZoom()
    {
        FitToWidth = true;
        _offset = SKPoint.Empty;
        Invalidate();
    }

    /// <summary>
    /// Called when the user starts a pan gesture.
    /// </summary>
    public void StartPan(float x, float y)
    {
        _isPanning = true;
        _lastPan = new SKPoint(x, y);
    }

    /// <summary>
    /// Called during a pan gesture to update the offset.
    /// </summary>
    public void ContinuePan(float x, float y)
    {
        if (!_isPanning) return;

        var dx = x - _lastPan.X;
        var dy = y - _lastPan.Y;
        _offset.X += dx;
        _offset.Y += dy;
        _lastPan = new SKPoint(x, y);
        Invalidate();
    }

    /// <summary>
    /// Called when a pan gesture ends.
    /// </summary>
    public void EndPan()
    {
        _isPanning = false;
    }

    /// <summary>
    /// Clean up the bitmap resource.
    /// </summary>
    public void ReleaseBitmap()
    {
        _bitmap?.Dispose();
        _bitmap = null;
        Invalidate();
    }
}
