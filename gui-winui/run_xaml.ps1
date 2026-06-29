try {
    $p = Start-Process -FilePath "C:\Users\rahmm\.nuget\packages\microsoft.windowsappsdk\1.5.240428000\tools\net472\XamlCompiler.exe" `
        -ArgumentList "D:\manga-panel-extractor\manga-reader-v2\gui-winui\obj\x64\Debug\net8.0-windows10.0.19041.0\input.json","D:\manga-panel-extractor\manga-reader-v2\gui-winui\obj\x64\Debug\net8.0-windows10.0.19041.0\output.json" `
        -NoNewWindow -RedirectStandardOutput temp_xaml_out.txt -RedirectStandardError temp_xaml_err.txt -Wait
    Write-Host "Exit code: $($p.ExitCode)"
    Write-Host "=== STDOUT ==="
    if (Test-Path temp_xaml_out.txt) { Get-Content temp_xaml_out.txt }
    Write-Host "=== STDERR ==="
    if (Test-Path temp_xaml_err.txt) { Get-Content temp_xaml_err.txt }
} catch {
    Write-Host "Error: $_"
}
