"""JSON-RPC 2.0 IPC server for the manga reader AI pipeline.

Runs on localhost:8500 and exposes methods for:
- vision.analyze_images
- vision.detect_important_pages
- vision.get_important_panels
- ocr.extract_text
- tts.synthesize
- tts.list_providers
- llm.generate
- pipeline.analyze_full (orchestrated end-to-end)
"""

from __future__ import annotations

import asyncio
import json
import logging
import signal
import traceback
from typing import Any, Callable, Coroutine

from manga_reader_ai.config import settings
from manga_reader_ai.models.schemas import (
    ImageData,
    JSONRPCRequest,
    JSONRPCResponse,
    LLMRequest,
    OCRRequest,
    SynthesisRequest,
    TTSOptions,
)
from manga_reader_ai.vision import GeminiAnalyzer
from manga_reader_ai.tts import KokoroEngine
from manga_reader_ai.tts.manager import TTSManager
from manga_reader_ai.ocr import MangaOCR
from manga_reader_ai.llm.client import create_llm_client

logger = logging.getLogger(__name__)


# ── JSON-RPC Helpers ───────────────────────────────────────────────────


def make_response(
    request_id: int | str | None,
    result: Any = None,
    error: dict[str, Any] | None = None,
) -> str:
    """Build a JSON-RPC 2.0 response string."""
    resp: dict[str, Any] = {"jsonrpc": "2.0", "id": request_id}
    if error:
        resp["error"] = error
    else:
        resp["result"] = result
    return json.dumps(resp, default=str) + "\n"


def make_error(
    request_id: int | str | None,
    code: int,
    message: str,
    data: Any = None,
) -> str:
    err: dict[str, Any] = {"code": code, "message": message}
    if data:
        err["data"] = data
    return make_response(request_id, error=err)


# Standard JSON-RPC error codes
PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603


# ── RPC Server ─────────────────────────────────────────────────────────


class RPCServer:
    """Asynchronous JSON-RPC 2.0 TCP server.

    Dispatches incoming requests to registered handler methods.
    """

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 8500,
    ) -> None:
        self.host = host
        self.port = port
        self._handlers: dict[str, Callable[..., Coroutine[Any, Any, Any]]] = {}
        self._server: asyncio.AbstractServer | None = None

        # Register core handlers immediately
        self._init_handlers()

    def _init_handlers(self) -> None:
        """Set up the default RPC method handlers."""
        # Vision
        self.register("vision.analyze_images", self._handle_vision_analyze)
        self.register("vision.detect_important_pages", self._handle_vision_detect_pages)
        self.register("vision.get_important_panels", self._handle_vision_get_panels)

        # OCR
        self.register("ocr.extract_text", self._handle_ocr_extract)

        # TTS
        self.register("tts.synthesize", self._handle_tts_synthesize)
        self.register("tts.list_providers", self._handle_tts_list_providers)

        # LLM
        self.register("llm.generate", self._handle_llm_generate)

        # Pipeline (orchestrated)
        self.register("pipeline.analyze_full", self._handle_pipeline_full)

        # Meta
        self.register("rpc.health", self._handle_health)
        self.register("rpc.list_methods", self._handle_list_methods)

    def register(
        self,
        method: str,
        handler: Callable[..., Coroutine[Any, Any, Any]],
    ) -> None:
        """Register an RPC method handler."""
        self._handlers[method] = handler

    # ── Server Lifecycle ─────────────────────────────────────────────

    async def start(self) -> None:
        """Start the TCP server."""
        self._server = await asyncio.start_server(
            self._handle_client,
            host=self.host,
            port=self.port,
            reuse_address=True,
            reuse_port=False,
        )
        addr = self._server.sockets[0].getsockname()
        logger.info(f"[RPC Server] Listening on {addr[0]}:{addr[1]}")

    async def serve_forever(self) -> None:
        """Run the server until cancelled."""
        if not self._server:
            await self.start()
        async with self._server:
            await self._server.serve_forever()

    async def shutdown(self) -> None:
        """Gracefully shut down the server."""
        if self._server:
            self._server.close()
            await self._server.wait_closed()
            logger.info("[RPC Server] Shut down")

    # ── Client Handler ───────────────────────────────────────────────

    async def _handle_client(
        self,
        reader: asyncio.StreamReader,
        writer: asyncio.StreamWriter,
    ) -> None:
        """Handle a single TCP connection — read JSON-RPC requests line by line."""
        peer = writer.get_extra_info("peername")
        logger.debug(f"[RPC] Connection from {peer}")
        try:
            while True:
                line = await reader.readline()
                if not line:
                    break  # Connection closed

                raw = line.decode("utf-8").strip()
                if not raw:
                    continue

                response = await self._dispatch(raw)
                if response:
                    writer.write(response.encode("utf-8"))
                    await writer.drain()
        except asyncio.CancelledError:
            pass
        except Exception as exc:
            logger.error(f"[RPC] Error handling client {peer}: {exc}")
        finally:
            try:
                writer.close()
                await writer.wait_closed()
            except Exception:
                pass
            logger.debug(f"[RPC] Connection closed: {peer}")

    # ── Dispatch ─────────────────────────────────────────────────────

    async def _dispatch(self, raw: str) -> str:
        """Parse a JSON-RPC request and dispatch to the appropriate handler."""
        # Parse JSON
        try:
            req_data = json.loads(raw)
        except json.JSONDecodeError as exc:
            return make_error(None, PARSE_ERROR, "Parse error", str(exc))

        # Validate request structure
        if not isinstance(req_data, dict) or req_data.get("jsonrpc") != "2.0":
            return make_error(
                req_data.get("id") if isinstance(req_data, dict) else None,
                INVALID_REQUEST,
                "Invalid Request: must be valid JSON-RPC 2.0",
            )

        request = JSONRPCRequest(**req_data)
        method = request.method
        params = request.params or {}
        req_id = request.id

        # Find handler
        handler = self._handlers.get(method)
        if handler is None:
            return make_error(
                req_id,
                METHOD_NOT_FOUND,
                f"Method '{method}' not found",
                {"available_methods": list(self._handlers.keys())},
            )

        # Call handler
        try:
            result = await handler(**params)
            return make_response(req_id, result=result)
        except TypeError as exc:
            return make_error(req_id, INVALID_PARAMS, f"Invalid params: {exc}", str(exc))
        except Exception as exc:
            logger.error(f"[RPC] Handler '{method}' failed: {exc}")
            return make_error(
                req_id,
                INTERNAL_ERROR,
                str(exc),
                traceback.format_exc() if logger.isEnabledFor(logging.DEBUG) else None,
            )

    # ── Handler Implementations ──────────────────────────────────────

    async def _handle_health(self) -> dict[str, Any]:
        return {"status": "ok", "service": "manga-reader-ai-pipeline"}

    async def _handle_list_methods(self) -> list[str]:
        return sorted(self._handlers.keys())

    async def _handle_vision_analyze(
        self,
        images: list[dict[str, Any]],
        character_profiles: list[dict[str, Any]] | None = None,
        prompt: str = "",
        instructions: str = "",
        detail: str = "low",
    ) -> dict[str, Any]:
        analyzer = GeminiAnalyzer()
        image_objs = [ImageData(**img) for img in images]
        profile_objs = (
            [ImageData(**p) for p in character_profiles] if character_profiles else None
        )
        result = await analyzer.analyze_images(
            pages=image_objs,
            character_profiles=profile_objs,
            prompt=prompt,
            instructions=instructions,
            detail=detail,
        )
        return result.model_dump()

    async def _handle_vision_detect_pages(
        self,
        pages: list[dict[str, Any]],
        profile_reference: list[dict[str, Any]] | None = None,
        chapter_reference: list[dict[str, Any]] | None = None,
        prompt: str = "",
        instructions: str = "",
    ) -> dict[str, Any]:
        analyzer = GeminiAnalyzer()
        pages_objs = [ImageData(**p) for p in pages]
        profile_objs = (
            [ImageData(**p) for p in profile_reference] if profile_reference else None
        )
        chapter_objs = (
            [ImageData(**p) for p in chapter_reference] if chapter_reference else None
        )
        result = await analyzer.detect_important_pages(
            pages=pages_objs,
            profile_reference=profile_objs,
            chapter_reference=chapter_objs,
            prompt=prompt,
            instructions=instructions,
        )
        return result.model_dump()

    async def _handle_vision_get_panels(
        self,
        panels: list[dict[str, Any]],
        prompt: str = "",
        instructions: str = "",
        character_profiles: list[dict[str, Any]] | None = None,
    ) -> dict[str, Any]:
        analyzer = GeminiAnalyzer()
        panel_objs = [ImageData(**p) for p in panels]
        profile_objs = (
            [ImageData(**p) for p in character_profiles] if character_profiles else None
        )
        result = await analyzer.get_important_panels(
            panels=panel_objs,
            prompt=prompt,
            instructions=instructions,
            character_profiles=profile_objs,
        )
        return result.model_dump()

    async def _handle_ocr_extract(
        self,
        images: list[dict[str, Any]],
        languages: list[str] | None = None,
        detail: bool = True,
    ) -> dict[str, Any]:
        ocr = MangaOCR(languages=languages)
        image_b64s = [img["data"] for img in images]
        batch_result = await ocr.extract_to_model(image_b64s)
        return batch_result.model_dump()

    async def _handle_tts_synthesize(
        self,
        text: str,
        options: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        tts_opts = TTSOptions(**(options or {}))
        engine = KokoroEngine(voice=tts_opts.voice, speed=tts_opts.speed)
        result = await engine.synthesize(text, voice=tts_opts.voice, speed=tts_opts.speed)
        return result.model_dump()

    async def _handle_tts_list_providers(self) -> dict[str, Any]:
        manager = TTSManager()
        return {"providers": manager.available_providers()}

    async def _handle_llm_generate(
        self,
        messages: list[dict[str, Any]],
        model: str = "",
        max_tokens: int = 4096,
        temperature: float = 0.7,
        response_format: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        client = create_llm_client()
        request = LLMRequest(
            messages=messages,
            model=model,
            max_tokens=max_tokens,
            temperature=temperature,
            response_format=response_format,
        )
        result = await client.generate(request)
        return result.model_dump()

    async def _handle_pipeline_full(
        self,
        images: list[dict[str, Any]],
        character_profiles: list[dict[str, Any]] | None = None,
        summarize: bool = True,
        detect_pages: bool = False,
    ) -> dict[str, Any]:
        """Orchestrated pipeline: analyse images and optionally detect important pages."""
        image_objs = [ImageData(**img) for img in images]
        profile_objs = (
            [ImageData(**p) for p in character_profiles] if character_profiles else None
        )
        analyzer = GeminiAnalyzer()
        results: dict[str, Any] = {}

        if summarize:
            analysis = await analyzer.analyze_images(
                pages=image_objs,
                character_profiles=profile_objs,
            )
            results["summary"] = analysis.model_dump()

        if detect_pages:
            page_detection = await analyzer.detect_important_pages(
                pages=image_objs,
                profile_reference=profile_objs,
            )
            results["important_pages"] = [p.model_dump() for p in page_detection.important_pages]

        return results


# ── Main Entry Point ────────────────────────────────────────────────────


async def main() -> None:
    """Run the RPC server."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    host = settings.server_host
    port = settings.server_port

    server = RPCServer(host=host, port=port)

    # Handle graceful shutdown
    stop_event = asyncio.Event()

    def _signal_handler() -> None:
        logger.info("[main] Shutdown signal received")
        stop_event.set()

    loop = asyncio.get_event_loop()
    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            loop.add_signal_handler(sig, _signal_handler)
        except NotImplementedError:
            # Windows doesn't support add_signal_handler
            pass

    await server.start()
    logger.info(f"[main] JSON-RPC server running on {host}:{port}")

    try:
        await stop_event.wait()
    except KeyboardInterrupt:
        pass
    finally:
        await server.shutdown()
        logger.info("[main] Server stopped")


if __name__ == "__main__":
    asyncio.run(main())
