"""Configuration for the manga reader AI pipeline.

Uses pydantic-settings to load from environment variables / .env file.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Literal

from pydantic import field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_prefix="MANGA_AI_",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # ── Server ──────────────────────────────────────────────────────────
    server_host: str = "127.0.0.1"
    server_port: int = 8500

    # ── Gemini Vision ──────────────────────────────────────────────────
    gemini_api_key: str = ""
    gemini_model: str = "gemini-2.0-flash"
    gemini_detail: Literal["low", "high"] = "low"

    # ── LLM ────────────────────────────────────────────────────────────
    llm_provider: Literal["gemini", "openai", "anthropic"] = "gemini"
    llm_model: str = "gemini-2.0-flash"
    llm_api_key: str = ""
    llm_max_tokens: int = 4096

    # ── TTS ────────────────────────────────────────────────────────────
    tts_provider: Literal["kokoro", "elevenlabs", "gemini"] = "kokoro"
    tts_voice: str = "af_heart"
    tts_speed: float = 1.0
    elevenlabs_api_key: str = ""
    elevenlabs_voice_id: str = "pNInz6obpgDQGcFmaJgB"

    # ── OCR ────────────────────────────────────────────────────────────
    ocr_languages: list[str] = ["ja", "en"]
    ocr_gpu: bool = True

    @field_validator("ocr_languages", mode="before")
    @classmethod
    def parse_ocr_languages(cls, v: Any) -> list[str]:
        """Accept both JSON arrays and comma-separated strings."""
        if isinstance(v, str):
            v = v.strip()
            if v.startswith("["):
                return json.loads(v)
            return [x.strip() for x in v.split(",") if x.strip()]
        return v

    # ── Output ─────────────────────────────────────────────────────────
    output_dir: Path = Path("output")
    tmp_dir: Path = Path("tmp")


settings = Settings()
