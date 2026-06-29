using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using MangaReader.Models;
using MangaReader.Services;

namespace MangaReader.ViewModels;

/// <summary>
/// Main view model providing navigation state and shared data
/// for the entire application.
/// </summary>
public partial class MainViewModel : ObservableObject
{
    private readonly IpcClient _ipcClient;
    private readonly CacheService _cacheService;

    public IpcClient IpcClient => _ipcClient;
    public CacheService CacheService => _cacheService;

    // ── Navigation state ──────────────────────────────────────────────

    [ObservableProperty]
    private string _currentPage = "Library";

    [ObservableProperty]
    private bool _isConnected;

    [ObservableProperty]
    private string _statusMessage = "Ready";

    // ── Library ────────────────────────────────────────────────────────

    [ObservableProperty]
    private ObservableCollection<Manga> _mangaCollection = new();

    [ObservableProperty]
    private string _searchQuery = string.Empty;

    [ObservableProperty]
    private Manga? _selectedManga;

    // ── Reader ─────────────────────────────────────────────────────────

    [ObservableProperty]
    private ObservableCollection<Page> _currentPages = new();

    [ObservableProperty]
    private int _currentPageIndex;

    [ObservableProperty]
    private string _readerTitle = string.Empty;

    [ObservableProperty]
    private bool _isFullScreen;

    // ── Settings ───────────────────────────────────────────────────────

    [ObservableProperty]
    private int _cacheSizeMb;

    [ObservableProperty]
    private bool _doublePageMode;

    [ObservableProperty]
    private bool _fitToWidth = true;

    // ── Commands ───────────────────────────────────────────────────────

    public MainViewModel(IpcClient ipcClient, CacheService cacheService)
    {
        _ipcClient = ipcClient;
        _cacheService = cacheService;

        _ipcClient.ConnectionStateChanged += (_, connected) =>
        {
            IsConnected = connected;
            StatusMessage = connected ? "Connected to engine" : "Disconnected";
        };

        RefreshCacheSize();
    }

    [RelayCommand]
    private async Task ConnectAsync()
    {
        try
        {
            StatusMessage = "Connecting...";
            await _ipcClient.ConnectAsync();
        }
        catch (Exception ex)
        {
            StatusMessage = $"Connection failed: {ex.Message}";
        }
    }

    [RelayCommand]
    private async Task LoadLibraryAsync()
    {
        try
        {
            StatusMessage = "Loading library...";
            var mangaList = await _ipcClient.GetLibraryAsync();
            MangaCollection = new ObservableCollection<Manga>(mangaList);

            // Apply search filter
            if (!string.IsNullOrWhiteSpace(SearchQuery))
            {
                var filtered = mangaList
                    .Where(m => m.Title.Contains(SearchQuery, StringComparison.OrdinalIgnoreCase)
                             || m.Author.Contains(SearchQuery, StringComparison.OrdinalIgnoreCase))
                    .ToList();
                MangaCollection = new ObservableCollection<Manga>(filtered);
            }

            StatusMessage = $"Library loaded — {MangaCollection.Count} titles";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Failed to load library: {ex.Message}";
        }
    }

    [RelayCommand]
    private void NavigateTo(string page)
    {
        CurrentPage = page;
        StatusMessage = $"Navigated to {page}";
    }

    [RelayCommand]
    private async Task OpenReaderAsync(Manga manga)
    {
        if (manga == null) return;

        SelectedManga = manga;
        ReaderTitle = manga.Title;
        CurrentPageIndex = 0;

        try
        {
            StatusMessage = $"Loading {manga.Title}...";
            var pages = await _ipcClient.GetPagesAsync(manga.Id, chapter: 1);
            CurrentPages = new ObservableCollection<Page>(pages);
            CurrentPage = "Reader";
            StatusMessage = $"Reading {manga.Title} — {pages.Count} pages";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Failed to load pages: {ex.Message}";
        }
    }

    [RelayCommand]
    private void NextPage()
    {
        if (CurrentPageIndex < CurrentPages.Count - 1)
            CurrentPageIndex++;
    }

    [RelayCommand]
    private void PreviousPage()
    {
        if (CurrentPageIndex > 0)
            CurrentPageIndex--;
    }

    [RelayCommand]
    private void ToggleFullScreen()
    {
        IsFullScreen = !IsFullScreen;
    }

    [RelayCommand]
    private void ClearCache()
    {
        _cacheService.ClearAll();
        RefreshCacheSize();
        StatusMessage = "Cache cleared";
    }

    [RelayCommand]
    private void RefreshCacheSize()
    {
        var bytes = _cacheService.GetCacheSizeBytes();
        CacheSizeMb = (int)(bytes / (1024 * 1024));
    }

    partial void OnSearchQueryChanged(string value)
    {
        _ = LoadLibraryAsync();
    }
}
