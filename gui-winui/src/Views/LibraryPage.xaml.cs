using MangaReader.Models;
using MangaReader.ViewModels;

namespace MangaReader.Views;

/// <summary>
/// Library page showing a grid of manga covers with search and filtering.
/// </summary>
public sealed partial class LibraryPage : Page
{
    private MainViewModel? _viewModel;

    public LibraryPage()
    {
        InitializeComponent();
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        _viewModel = DataContext as MainViewModel;
        if (_viewModel != null && _viewModel.MangaCollection.Count == 0)
        {
            _viewModel.LoadLibraryCommand.Execute(null);
        }
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        // Release any cached cover bitmaps held by MangaCover controls
    }

    private void OnCoverPointerEntered(object sender, PointerRoutedEventArgs e)
    {
        if (sender is Border border)
        {
            border.Scale(new System.Numerics.Vector3(1.03f, 1.03f, 1f),
                         (float)border.ActualWidth / 2, (float)border.ActualHeight / 2);
        }
    }

    private void OnCoverPointerExited(object sender, PointerRoutedEventArgs e)
    {
        if (sender is Border border)
        {
            border.Scale(System.Numerics.Vector3.One,
                         (float)border.ActualWidth / 2, (float)border.ActualHeight / 2);
        }
    }

    private void OnCoverPointerPressed(object sender, PointerRoutedEventArgs e)
    {
        if (sender is Border border && border.DataContext is Manga manga)
        {
            _viewModel?.OpenReaderCommand.Execute(manga);
        }
    }
}
