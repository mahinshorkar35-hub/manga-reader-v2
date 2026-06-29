"""LLM client — async abstraction over Gemini, OpenAI, and Anthropic.

Ported from vision_analysis.py completions() helper pattern.
"""

from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from typing import Any

from manga_reader_ai.config import settings
from manga_reader_ai.models.schemas import LLMRequest, LLMResponse

logger = logging.getLogger(__name__)


class LLMClient(ABC):
    """Abstract base for LLM providers."""

    @abstractmethod
    async def generate(self, request: LLMRequest) -> LLMResponse:
        ...


# ── Gemini ─────────────────────────────────────────────────────────────


class GeminiLLMClient(LLMClient):
    """Google Gemini LLM client."""

    def __init__(self, api_key: str | None = None, model: str | None = None) -> None:
        self.api_key = api_key or settings.llm_api_key or settings.gemini_api_key
        self.model_name = model or settings.llm_model or settings.gemini_model
        self._client: Any = None

    @property
    def client(self) -> Any:
        if self._client is None:
            from google import genai

            self._client = genai.Client(api_key=self.api_key)
        return self._client

    async def generate(self, request: LLMRequest) -> LLMResponse:
        import json

        from google.genai import types

        model = request.model or self.model_name
        config: dict[str, Any] = {"max_output_tokens": request.max_tokens}

        # Extract system instruction if first message has role "system"
        system_instruction: str | None = None
        contents_messages: list[dict[str, Any]] = []
        for msg in request.messages:
            if msg.get("role") == "system":
                system_instruction = msg.get("content", "")
            else:
                contents_messages.append(msg)

        # Convert OpenAI-format messages to Gemini format
        gemini_contents: list[dict[str, Any]] = []
        for msg in contents_messages:
            role = msg.get("role", "user")
            content = msg.get("content", "")
            if isinstance(content, str):
                gemini_contents.append(
                    {"role": "user" if role == "user" else "model", "parts": [types.Part.from_text(text=content)]}
                )
            elif isinstance(content, list):
                # Multi-part content (text + images)
                parts: list[Any] = []
                for part in content:
                    if part.get("type") == "text":
                        parts.append(types.Part.from_text(text=part["text"]))
                    elif part.get("type") == "image_url":
                        img_url = part["image_url"]["url"]
                        if img_url.startswith("data:"):
                            import base64
                            import re

                            match = re.match(r"data:image/\w+;base64,(.+)", img_url)
                            if match:
                                img_bytes = base64.b64decode(match.group(1))
                                mime = img_url.split(";")[0].split(":")[1]
                                parts.append(
                                    types.Part.from_bytes(data=img_bytes, mime_type=mime)
                                )
                gemini_contents.append(
                    {"role": "user" if role == "user" else "model", "parts": parts}
                )

        if not gemini_contents:
            gemini_contents = [{"role": "user", "parts": [types.Part.from_text(text="")]}]

        fn = self.client.models.generate_content
        kwargs: dict[str, Any] = {
            "model": model,
            "contents": gemini_contents,
            "config": config,
        }
        if system_instruction:
            kwargs["config"]["system_instruction"] = system_instruction

        def _run() -> Any:
            return fn(**kwargs)

        response = await asyncio.to_thread(_run)

        text = response.text if hasattr(response, "text") else str(response)
        total_tokens = 0
        if hasattr(response, "usage_metadata") and response.usage_metadata:
            total_tokens = getattr(response.usage_metadata, "total_token_count", 0)

        return LLMResponse(
            text=text,
            total_tokens=total_tokens,
            model=model,
            provider="gemini",
        )


# ── OpenAI ──────────────────────────────────────────────────────────────


class OpenAILLMClient(LLMClient):
    """OpenAI-compatible LLM client."""

    def __init__(
        self,
        api_key: str | None = None,
        model: str | None = None,
        base_url: str | None = None,
    ) -> None:
        self.api_key = api_key or settings.llm_api_key
        self.model_name = model or settings.llm_model or "gpt-4o"
        self.base_url = base_url
        self._client: Any = None

    @property
    def client(self) -> Any:
        if self._client is None:
            from openai import AsyncOpenAI

            kwargs: dict[str, Any] = {"api_key": self.api_key}
            if self.base_url:
                kwargs["base_url"] = self.base_url
            self._client = AsyncOpenAI(**kwargs)
        return self._client

    async def generate(self, request: LLMRequest) -> LLMResponse:
        model = request.model or self.model_name

        kwargs: dict[str, Any] = {
            "model": model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        }
        if request.response_format:
            kwargs["response_format"] = request.response_format

        response = await self.client.chat.completions.create(**kwargs)

        text = response.choices[0].message.content or ""
        total_tokens = response.usage.total_tokens if response.usage else 0

        return LLMResponse(
            text=text,
            total_tokens=total_tokens,
            model=model,
            provider="openai",
        )


# ── Anthropic ───────────────────────────────────────────────────────────


class AnthropicLLMClient(LLMClient):
    """Anthropic Claude LLM client."""

    def __init__(
        self,
        api_key: str | None = None,
        model: str | None = None,
    ) -> None:
        self.api_key = api_key or settings.llm_api_key
        self.model_name = model or settings.llm_model or "claude-3-5-sonnet-20241022"
        self._client: Any = None

    @property
    def client(self) -> Any:
        if self._client is None:
            from anthropic import AsyncAnthropic

            self._client = AsyncAnthropic(api_key=self.api_key)
        return self._client

    async def generate(self, request: LLMRequest) -> LLMResponse:
        model = request.model or self.model_name

        # Convert OpenAI-format to Anthropic format
        system: str | None = None
        messages: list[dict[str, Any]] = []
        for msg in request.messages:
            if msg.get("role") == "system":
                system = msg.get("content", "")
            else:
                messages.append({"role": msg["role"], "content": msg["content"]})

        kwargs: dict[str, Any] = {
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
        }
        if system:
            kwargs["system"] = system
        if request.temperature:
            kwargs["temperature"] = request.temperature

        response = await self.client.messages.create(**kwargs)

        text = "".join(block.text for block in response.content if block.type == "text")
        total_tokens = (
            (response.usage.input_tokens + response.usage.output_tokens)
            if response.usage
            else 0
        )

        return LLMResponse(
            text=text,
            total_tokens=total_tokens,
            model=model,
            provider="anthropic",
        )


# ── Factory ─────────────────────────────────────────────────────────────


def create_llm_client(
    provider: str | None = None,
    api_key: str | None = None,
    model: str | None = None,
) -> LLMClient:
    """Create the appropriate LLM client based on configuration."""
    provider = (provider or settings.llm_provider).lower()

    if provider == "gemini":
        return GeminiLLMClient(api_key=api_key, model=model)
    elif provider == "openai":
        return OpenAILLMClient(api_key=api_key, model=model)
    elif provider == "anthropic":
        return AnthropicLLMClient(api_key=api_key, model=model)
    else:
        raise ValueError(f"Unsupported LLM provider: {provider}")
