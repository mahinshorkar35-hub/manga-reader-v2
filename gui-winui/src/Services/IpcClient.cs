using System.Net.WebSockets;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace MangaReader.Services;

/// <summary>
/// JSON-RPC client that communicates with the Rust manga engine
/// over a WebSocket connection (stdin/stdout fallback available).
/// </summary>
public class IpcClient : IAsyncDisposable
{
    private ClientWebSocket? _webSocket;
    private readonly string _endpoint;
    private int _requestId;
    private readonly Dictionary<int, TaskCompletionSource<JObject>> _pendingRequests = new();

    private const int DefaultTimeoutMs = 30_000;

    public event EventHandler<string>? NotificationReceived;
    public event EventHandler<bool>? ConnectionStateChanged;

    public bool IsConnected => _webSocket?.State == WebSocketState.Open;

    public IpcClient(string endpoint = "ws://127.0.0.1:9721")
    {
        _endpoint = endpoint;
    }

    /// <summary>
    /// Connect to the Rust engine's WebSocket JSON-RPC endpoint.
    /// </summary>
    public async Task ConnectAsync(CancellationToken ct = default)
    {
        _webSocket?.Dispose();
        _webSocket = new ClientWebSocket();

        try
        {
            await _webSocket.ConnectAsync(new Uri(_endpoint), ct);
            ConnectionStateChanged?.Invoke(this, true);
            _ = ReceiveLoopAsync(CancellationToken.None);
        }
        catch (Exception ex)
        {
            ConnectionStateChanged?.Invoke(this, false);
            throw new InvalidOperationException($"Failed to connect to Rust engine at {_endpoint}", ex);
        }
    }

    /// <summary>
    /// Disconnect cleanly from the engine.
    /// </summary>
    public async Task DisconnectAsync()
    {
        if (_webSocket is { State: WebSocketState.Open })
        {
            try
            {
                await _webSocket.CloseAsync(
                    WebSocketCloseStatus.NormalClosure, "Client closing", CancellationToken.None);
            }
            catch { /* ignore close errors */ }
        }

        _webSocket?.Dispose();
        _webSocket = null;
        ConnectionStateChanged?.Invoke(this, false);

        // Fail all pending requests
        foreach (var tcs in _pendingRequests.Values)
            tcs.TrySetCanceled();
        _pendingRequests.Clear();
    }

    /// <summary>
    /// Send a JSON-RPC method call and await the result.
    /// </summary>
    public async Task<JObject> CallAsync(string method, JObject? parameters = null, int timeoutMs = DefaultTimeoutMs)
    {
        EnsureConnected();

        var id = Interlocked.Increment(ref _requestId);
        var request = new JObject
        {
            ["jsonrpc"] = "2.0",
            ["id"] = id,
            ["method"] = method,
            ["params"] = parameters ?? new JObject()
        };

        var tcs = new TaskCompletionSource<JObject>();
        lock (_pendingRequests)
            _pendingRequests[id] = tcs;

        try
        {
            var json = request.ToString(Formatting.None);
            var bytes = Encoding.UTF8.GetBytes(json);
            await _webSocket!.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, CancellationToken.None);

            using var cts = new CancellationTokenSource(timeoutMs);
            using var registration = cts.Token.Register(() => tcs.TrySetCanceled());
            return await tcs.Task;
        }
        finally
        {
            lock (_pendingRequests)
                _pendingRequests.Remove(id);
        }
    }

    /// <summary>
    /// Send a notification (no response expected).
    /// </summary>
    public async Task NotifyAsync(string method, JObject? parameters = null)
    {
        EnsureConnected();

        var request = new JObject
        {
            ["jsonrpc"] = "2.0",
            ["method"] = method,
            ["params"] = parameters ?? new JObject()
        };

        var json = request.ToString(Formatting.None);
        var bytes = Encoding.UTF8.GetBytes(json);
        await _webSocket!.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, CancellationToken.None);
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        var buffer = new byte[1024 * 64];
        var sb = new StringBuilder();

        try
        {
            while (_webSocket?.State == WebSocketState.Open && !ct.IsCancellationRequested)
            {
                var result = await _webSocket.ReceiveAsync(new ArraySegment<byte>(buffer), ct);
                sb.Append(Encoding.UTF8.GetString(buffer, 0, result.Count));

                if (result.EndOfMessage)
                {
                    ProcessMessage(sb.ToString());
                    sb.Clear();
                }
            }
        }
        catch (WebSocketException)
        {
            // Connection lost
        }
        catch (OperationCanceledException) { }
        finally
        {
            if (_webSocket?.State != WebSocketState.Open)
                ConnectionStateChanged?.Invoke(this, false);
        }
    }

    private void ProcessMessage(string raw)
    {
        try
        {
            var msg = JObject.Parse(raw);

            // Check for notification (no id field)
            if (msg["id"] == null || msg["id"].Type == JTokenType.Null)
            {
                var method = msg["method"]?.ToString() ?? "unknown";
                NotificationReceived?.Invoke(this, method);
                return;
            }

            var id = msg["id"].Value<int>();
            lock (_pendingRequests)
            {
                if (_pendingRequests.TryGetValue(id, out var tcs))
                {
                    if (msg["error"] != null)
                        tcs.TrySetException(new InvalidOperationException(msg["error"]["message"]?.ToString()));
                    else
                        tcs.TrySetResult(msg["result"] as JObject ?? msg);
                }
            }
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"IPC parse error: {ex.Message}");
        }
    }

    private void EnsureConnected()
    {
        if (_webSocket is not { State: WebSocketState.Open })
            throw new InvalidOperationException("Not connected to Rust engine. Call ConnectAsync first.");
    }

    /// <summary>
    /// Load the list of manga from the engine.
    /// </summary>
    public async Task<List<Models.Manga>> GetLibraryAsync()
    {
        var result = await CallAsync("get_library");
        return result["items"]?.ToObject<List<Models.Manga>>() ?? new List<Models.Manga>();
    }

    /// <summary>
    /// Request pages for a specific manga chapter from the engine.
    /// </summary>
    public async Task<List<Models.Page>> GetPagesAsync(string mangaId, int chapter)
    {
        var result = await CallAsync("get_pages", new JObject
        {
            ["manga_id"] = mangaId,
            ["chapter"] = chapter
        });
        return result["pages"]?.ToObject<List<Models.Page>>() ?? new List<Models.Page>();
    }

    /// <summary>
    /// Send reading progress update to the engine.
    /// </summary>
    public async Task UpdateProgressAsync(Models.ReadingProgress progress)
    {
        await NotifyAsync("update_progress", JObject.FromObject(progress));
    }

    public async ValueTask DisposeAsync()
    {
        await DisconnectAsync();
        GC.SuppressFinalize(this);
    }
}
