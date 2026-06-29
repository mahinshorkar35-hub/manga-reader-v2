using MangaReader.ViewModels;
using MangaReader.Views;

namespace MangaReader.Views;

/// <summary>
/// Main application window with navigation sidebar and content frame.
/// Registers navigation converters and handles page switching.
/// </summary>
public sealed partial class MainWindow : Window
{
    private readonly MainViewModel _viewModel;

    public MainWindow(MainViewModel viewModel)
    {
        _viewModel = viewModel;
        InitializeComponent();

        // Register converters for the sidebar
        Resources.Add("NotEqualConverter", new NotEqualConverter());
        Resources.Add("BoolToConnectionColor", new BoolToConnectionColorConverter());
        Resources.Add("BoolToConnectionText", new BoolToConnectionTextConverter());

        // Set data context
        this.DataContext = _viewModel;

        // Navigate to library on startup
        NavigateToPage("Library");

        // Listen for navigation changes from the ViewModel
        _viewModel.PropertyChanged += (s, e) =>
        {
            if (e.PropertyName == nameof(MainViewModel.CurrentPage))
            {
                NavigateToPage(_viewModel.CurrentPage);
            }
        };
    }

    private void NavigateToPage(string page)
    {
        var frame = ContentFrame;

        switch (page)
        {
            case "Library":
                frame.Navigate(typeof(LibraryPage), _viewModel);
                break;
            case "Reader":
                frame.Navigate(typeof(ReaderPage), _viewModel);
                break;
            case "Settings":
                frame.Navigate(typeof(SettingsPage), _viewModel);
                break;
        }
    }

    private void OnContentFrameNavigated(object sender, Microsoft.UI.Xaml.Navigation.NavigationEventArgs e)
    {
        // Update title or breadcrumbs if needed
    }
}

