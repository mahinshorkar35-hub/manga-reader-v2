using Microsoft.Extensions.DependencyInjection;
using MangaReader.Services;
using MangaReader.ViewModels;
using MangaReader.Views;

namespace MangaReader;

/// <summary>
/// Application entry point. Configures dependency injection
/// and launches the main window.
/// </summary>
public partial class App : Application
{
    public static IServiceProvider Services { get; private set; } = null!;

    public App()
    {
        InitializeComponent();

        Services = ConfigureServices();
    }

    protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
    {
        var mainWindow = Services.GetRequiredService<MainWindow>();
        mainWindow.Activate();
    }

    private static IServiceProvider ConfigureServices()
    {
        var services = new ServiceCollection();

        // Services
        services.AddSingleton<IpcClient>();
        services.AddSingleton<CacheService>();

        // ViewModels
        services.AddSingleton<MainViewModel>();

        // Views
        services.AddTransient<MainWindow>();
        services.AddTransient<LibraryPage>();
        services.AddTransient<ReaderPage>();
        services.AddTransient<SettingsPage>();

        return services.BuildServiceProvider();
    }
}
