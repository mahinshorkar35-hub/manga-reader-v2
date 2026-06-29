"""TTS provider abstraction — ported from tts_manager.py.

Supports Kokoro, ElevenLabs, and Gemini TTS providers with a unified async interface.
"""

from __future__ import annotations

import asyncio
import base64
import io
import logging
from abc import ABC, abstractmethod
from typing import Any

from manga_reader_ai.config import settings
from manga_reader_ai.models.schemas import SynthesisResult, TTSOptions

logger = logging.getLogger(__name__)


# ── Abstract Provider ──────────────────────────────────────────────────


class TTSProvider(ABC):
    """Abstract base class for TTS providers."""

    @abstractmethod
    async def synthesize(self, text: str, options: TTSOptions | None = None) -> SynthesisResult:
        ...


# ── Kokoro Provider ────────────────────────────────────────────────────


class KokoroTTSProvider(TTSProvider):
    """Kokoro local TTS provider.

    Ported from tts_manager.py's KokoroTTS class and video_director_kokoro.
    """

    def __init__(self, voice: str = "af_heart", speed: float = 1.0) -> None:
        self.default_voice = voice
        self.default_speed = speed
        self._pipeline: Any = None

    @property
    def pipeline(self) -> Any:
        if self._pipeline is None:
            from kokoro import KPipeline

            self._pipeline = KPipeline(lang_code="a")
        return self._pipeline

    async def synthesize(
        self, text: str, options: TTSOptions | None = None
    ) -> SynthesisResult:
        import numpy as np
        import soundfile as sf

        opts = options or TTSOptions()
        voice = opts.voice or self.default_voice
        speed = max(0.5, float(opts.speed or self.default_speed))
        pipe = self.pipeline

        def _run() -> tuple[bytes, float, int]:
            gen = pipe(text, voice=voice, speed=speed)
            all_audio: list[np.ndarray] = []
            sr = 24000
            for result in gen:
                if hasattr(result, "audio"):
                    chunk = np.array(
                        result.audio.cpu().numpy()
                        if hasattr(result.audio, "cpu")
                        else result.audio,
                        dtype=np.float32,
                    ).squeeze()
                elif isinstance(result, tuple):
                    chunk = result[0]
                    sr = result[1] if len(result) >= 2 else sr
                    chunk = np.array(chunk, dtype=np.float32).squeeze()
                else:
                    continue
                if chunk.size > 0:
                    all_audio.append(chunk)

            if not all_audio:
                raise RuntimeError("Kokoro returned empty audio")

            audio = np.concatenate(all_audio) if len(all_audio) > 1 else all_audio[0]

            # Normalise
            peak = float(np.max(np.abs(audio)))
            if peak > 0:
                audio = (audio / max(peak, 1e-9)) * min(0.9, peak)

            duration = float(len(audio) / float(sr))
            wav_buf = io.BytesIO()
            sf.write(wav_buf, audio, int(sr), format="WAV")
            return wav_buf.getvalue(), duration, int(sr)

        wav_bytes, duration, sr = await asyncio.to_thread(_run)

        return SynthesisResult(
            audio_bytes=base64.b64encode(wav_bytes).decode("ascii"),
            duration_seconds=duration,
            sample_rate=sr,
            provider="kokoro",
        )


# ── ElevenLabs Provider ────────────────────────────────────────────────


class ElevenLabsTTSProvider(TTSProvider):
    """ElevenLabs cloud TTS provider.

    Ported from tts_manager.py's ElevenLabstts class.
    """

    def __init__(self, api_key: str = "", voice_id: str = "pNInz6obpgDQGcFmaJgB") -> None:
        self.api_key = api_key or settings.elevenlabs_api_key
        self.voice_id = voice_id or settings.elevenlabs_voice_id
        self._client: Any = None

    @property
    def client(self) -> Any:
        if self._client is None:
            from elevenlabs.client import AsyncElevenLabs

            self._client = AsyncElevenLabs(api_key=self.api_key)
        return self._client

    async def synthesize(
        self, text: str, options: TTSOptions | None = None
    ) -> SynthesisResult:
        opts = options or TTSOptions()
        voice_id = opts.voice or self.voice_id

        audio_bytes_io = io.BytesIO()
        async for audio_bytes in self.client.text_to_speech.convert(
            text=text,
            voice_id=voice_id,
        ):
            audio_bytes_io.write(audio_bytes)

        audio_bytes_io.seek(0)
        raw = audio_bytes_io.read()

        return SynthesisResult(
            audio_bytes=base64.b64encode(raw).decode("ascii"),
            duration_seconds=0.0,  # ElevenLabs doesn't return duration directly
            sample_rate=44100,
            provider="elevenlabs",
        )


# ── Gemini TTS (stub) ─────────────────────────────────────────────────


class GeminiTTSProvider(TTSProvider):
    """Gemini TTS stub — not natively supported.

    Ported from tts_manager.py's GeminiTTS class.
    """

    async def synthesize(
        self, text: str, options: TTSOptions | None = None
    ) -> SynthesisResult:
        raise NotImplementedError(
            "Gemini does not support TTS natively. Use Kokoro or ElevenLabs instead."
        )


# ── Provider Registry / Manager ────────────────────────────────────────


class TTSManager:
    """Manages TTS providers and dispatches synthesis requests.

    Ported from tts_manager.py's TTSManager class.
    """

    def __init__(self) -> None:
        self._providers: dict[str, TTSProvider] = {}
        self._current: str | None = None
        self._register_defaults()

    def _register_defaults(self) -> None:
        self._providers["kokoro"] = KokoroTTSProvider(
            voice=settings.tts_voice, speed=settings.tts_speed
        )
        if settings.elevenlabs_api_key:
            self._providers["elevenlabs"] = ElevenLabsTTSProvider(
                api_key=settings.elevenlabs_api_key,
                voice_id=settings.elevenlabs_voice_id,
            )
        self._providers["gemini"] = GeminiTTSProvider()
        self._current = settings.tts_provider or "kokoro"

    def register(self, name: str, provider: TTSProvider) -> None:
        self._providers[name] = provider

    def set_provider(self, name: str) -> None:
        if name not in self._providers:
            raise ValueError(
                f"Unknown TTS provider '{name}'. "
                f"Available: {list(self._providers.keys())}"
            )
        self._current = name

    def get_provider(self, name: str | None = None) -> TTSProvider:
        name = name or self._current
        if not name:
            raise ValueError("No TTS provider selected")
        if name not in self._providers:
            raise ValueError(f"Provider '{name}' not configured")
        return self._providers[name]

    async def synthesize(
        self, text: str, options: TTSOptions | None = None
    ) -> SynthesisResult:
        provider = self.get_provider()
        return await provider.synthesize(text, options)

    def available_providers(self) -> list[str]:
        return list(self._providers.keys())

    def is_configured(self, name: str) -> bool:
        return name in self._providers
