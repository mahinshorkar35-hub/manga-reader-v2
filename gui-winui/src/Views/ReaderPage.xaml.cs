using SkiaSharp;
using MangaReader.Controls;
using MangaReader.Models;
using MangaReader.Services;
using MangaReader.ViewModels;

namespace MangaReader.Views;

/// <summary>
/// Full-screen reading view with SkiaSharp GPU page rendering,
/// tap-to-turn navigation, and zoom/pan support.
/// </summary>
public sealed partial class ReaderPage : Page
{
    private MainViewModel? _viewModel;
    private readonly CacheService _cacheService;
    private IDispatcherTimer? _uiHideTimer;

    public ReaderPage()
    {
        InitializeComponent();
        _cacheService = App.Services.GetRequiredService<CacheService>();
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        _viewModel = DataContext as MainViewModel;
        _viewModel!.PropertyChanged += OnViewModelPropertyChanged;

        // Start auto-hide timer for UI chrome
        _uiHideTimer = DispatcherQueue.CreateTimer();
        _uiHideTimer.Interval = TimeSpan.FromSeconds(3);
        _uiHideTimer.Tick += (s, args) => { HideChrome(); };

        // Load the current page
        LoadCurrentPage();

        // Focus for keyboard events
        this.Focus(FocusState.Programmatic);
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        if (_viewModel != null)
            _viewModel.PropertyChanged -= OnViewModelPropertyChanged;
        _uiHideTimer?.Stop();
        PageRenderer.ReleaseBitmap();
    }

    private void OnViewModelPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(MainViewModel.CurrentPageIndex))
        {
            LoadCurrentPage();
            ShowChrome();
        }
    }

    private async void LoadCurrentPage()
    {
        if (_viewModel == null || _viewModel.CurrentPages.Count == 0)
            return;

        var page = _viewModel.CurrentPages[_viewModel.CurrentPageIndex];
        var mangaId = _viewModel.SelectedManga?.Id ?? string.Empty;

        // Try cache first
        var bitmap = _cacheService.GetThumbnail(mangaId, 1, page.Index);
        if (bitmap != null)
        {
            PageRenderer.Bitmap = bitmap;
            return;
        }

        // Load from engine via image source
        if (!string.IsNullOrEmpty(page.ImageSource))
        {
            try
            {
                // Load image from path — for a real app this would stream
                // the decoded bitmap from the Rust engine over IPC
                if (File.Exists(page.ImageSource))
                {
                    using var stream = File.OpenRead(page.ImageSource);
                    bitmap = SKBitmap.Decode(stream);
                    if (bitmap != null)
                    {
                        _cacheService.StoreThumbnail(mangaId, 1, page.Index, bitmap);
                        PageRenderer.Bitmap = bitmap;
                        return;
                    }
                }
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"Failed to load page: {ex.Message}");
            }
        }

        // Fallback: try loading from a local archive directory
        var fallbackDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "MangaReader", "pages", mangaId, "ch1");

        var fallbackPath = Path.Combine(fallbackDir, $"page_{page.Index:D4}.png");
        if (File.Exists(fallbackPath))
        {
            using var stream = File.OpenRead(fallbackPath);
            bitmap = SKBitmap.Decode(stream);
            if (bitmap != null)
            {
                _cacheService.StoreThumbnail(mangaId, 1, page.Index, bitmap);
                PageRenderer.Bitmap = bitmap;
                return;
            }
        }

        // Nothing — show placeholder
        PageRenderer.Bitmap = null;
    }

    private void OnPageTapped(object sender, Microsoft.UI.Xaml.Input.TappedRoutedEventArgs e)
    {
        var position = e.GetPosition(PageRenderer);
        var width = PageRenderer.ActualWidth;

        // Tap left third = previous page, right third = next page
        if (position.X < width / 3)
        {
            _viewModel?.PreviousPageCommand.Execute(null);
        }
        else if (position.X > width * 2 / 3)
        {
            _viewModel?.NextPageCommand.Execute(null);
        }
        else
        {
            // Middle tap — toggle chrome
            ToggleChrome();
        }

        ShowChrome();
    }

    private void OnPageDoubleTapped(object sender, Microsoft.UI.Xaml.Input.DoubleTappedRoutedEventArgs e)
    {
        PageRenderer.ResetZoom();
        ShowChrome();
    }

    private void OnPointerWheelChanged(object sender, Microsoft.UI.Xaml.Input.PointerRoutedEventArgs e)
    {
        var delta = e.GetCurrentPoint(PageRenderer).Properties.MouseWheelDelta;
        if (delta > 0)
            PageRenderer.ZoomIn();
        else if (delta < 0)
            PageRenderer.ZoomOut();
        ShowChrome();
    }

    private void OnPageKeyDown(object sender, Microsoft.UI.Xaml.Input.KeyRoutedEventArgs e)
    {
        switch (e.Key)
        {
            case Windows.System.VirtualKey.Right:
            case Windows.System.VirtualKey.Down:
            case Windows.System.VirtualKey.Space:
                _viewModel?.NextPageCommand.Execute(null);
                break;
            case Windows.System.VirtualKey.Left:
            case Windows.System.VirtualKey.Up:
                _viewModel?.PreviousPageCommand.Execute(null);
                break;
            case Windows.System.VirtualKey.F:
            case Windows.System.VirtualKey.Escape:
                _viewModel?.ToggleFullScreenCommand.Execute(null);
                break;
        }
        ShowChrome();
    }

    private void ShowChrome()
    {
        TopBar.Visibility = Visibility.Visible;
        BottomBar.Visibility = Visibility.Visible;
        _uiHideTimer?.Start();
    }

    private void HideChrome()
    {
        TopBar.Visibility = Visibility.Collapsed;
        BottomBar.Visibility = Visibility.Collapsed;
    }

    private void ToggleChrome()
    {
        if (TopBar.Visibility == Visibility.Visible)
            HideChrome();
        else
            ShowChrome();
    }
}
