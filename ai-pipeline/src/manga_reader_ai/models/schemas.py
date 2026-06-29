"""Pydantic schemas for all data flowing through the AI pipeline."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from pydantic import BaseModel, Field


# ── Images ──────────────────────────────────────────────────────────────


class ImageData(BaseModel):
    """A base64-encoded image with metadata."""

    data: str = Field(description="Base64-encoded image bytes")
    mime_type: str = Field(default="image/png", description="MIME type of the image")
    index: int = Field(default=0, description="Zero-based index in the sequence")


# ── Vision / Analysis ───────────────────────────────────────────────────


class AnalysisRequest(BaseModel):
    """Request payload for manga page analysis."""

    images: list[ImageData] = Field(description="Pages or panels to analyse")
    character_profiles: list[ImageData] = Field(
        default_factory=list, description="Optional character profile reference images"
    )
    prompt: str = Field(default="", description="Custom prompt override")
    instructions: str = Field(default="", description="System instructions override")
    detail: str = Field(default="low", description="Image detail level: 'low' or 'high'")


class ImportantPage(BaseModel):
    """A detected important page (character profile or chapter start)."""

    image_index: int
    type: str = Field(description="'profile' or 'chapter'")


class ImportantPanel(BaseModel):
    """A panel identified as relevant to a given text."""

    panel_index: int


class Citation(BaseModel):
    """An inline citation referencing an image index."""

    image_index: int


class AnalysisResult(BaseModel):
    """Result from a vision analysis call."""

    text: str = Field(default="", description="Generated summary/analysis text")
    total_tokens: int = Field(default=0, description="Token count used")
    important_pages: list[ImportantPage] = Field(default_factory=list)
    important_panels: list[int] = Field(default_factory=list)
    raw_response: str = Field(default="", description="Raw model response for debugging")


# ── OCR ─────────────────────────────────────────────────────────────────


class OCRRequest(BaseModel):
    """Request payload for OCR text extraction."""

    images: list[ImageData] = Field(description="Images to extract text from")
    languages: list[str] = Field(default=["ja", "en"])
    detail: bool = Field(default=True, description="Include bbox and confidence")


class OCRResultItem(BaseModel):
    """A single OCR-extracted text region."""

    text: str
    confidence: float = 0.0
    bbox: list[list[float]] = Field(default_factory=list)


class OCRResult(BaseModel):
    """Result from OCR extraction on a single image."""

    image_index: int
    items: list[OCRResultItem] = Field(default_factory=list)

    @property
    def text(self) -> str:
        return " ".join(item.text for item in self.items)


class OCRBatchResult(BaseModel):
    """Batch OCR result."""

    results: list[OCRResult]


# ── TTS ─────────────────────────────────────────────────────────────────


class TTSOptions(BaseModel):
    """Options for TTS synthesis."""

    provider: str = Field(default="kokoro", description="TTS provider to use")
    voice: str = Field(default="af_heart", description="Voice identifier")
    speed: float = Field(default=1.0, ge=0.5, le=3.0, description="Speech rate")


class SynthesisRequest(BaseModel):
    """Request payload for TTS."""

    text: str = Field(description="Text to synthesise")
    options: TTSOptions = Field(default_factory=TTSOptions)


class SynthesisResult(BaseModel):
    """Result from TTS synthesis."""

    audio_bytes: str = Field(default="", description="Base64-encoded WAV audio data")
    duration_seconds: float = 0.0
    sample_rate: int = 24000
    provider: str = "kokoro"


# ── Script / Movie ──────────────────────────────────────────────────────


class ScriptSegment(BaseModel):
    """A single segment of a movie script containing narration and images."""

    text: str = ""
    images: list[str] = Field(default_factory=list, description="Base64 images (scaled)")
    images_unscaled: list[str] = Field(
        default_factory=list, description="Base64 images (full resolution)"
    )
    page_indices: list[int] = Field(default_factory=list)


class MangaSegment(BaseModel):
    """A segment in the manga pipeline linking pages to narration."""

    text: str = ""
    images_b64: list[str] = Field(default_factory=list)
    image_indices: list[int] = Field(default_factory=list)
    duration_seconds: float = 0.0


# ── LLM ─────────────────────────────────────────────────────────────────


class LLMRequest(BaseModel):
    """Request payload for LLM completion."""

    messages: list[dict[str, Any]] = Field(
        description="Chat messages in OpenAI-compatible format"
    )
    model: str = Field(default="", description="Model override")
    max_tokens: int = Field(default=4096)
    temperature: float = Field(default=0.7, ge=0.0, le=2.0)
    response_format: dict[str, Any] | None = Field(
        default=None, description="Optional structured output format"
    )


class LLMResponse(BaseModel):
    """Response from an LLM completion."""

    text: str = ""
    total_tokens: int = 0
    model: str = ""
    provider: str = ""


# ── Server Messages ──────────────────────────────────────────────────────


class JSONRPCRequest(BaseModel):
    """JSON-RPC 2.0 request."""

    jsonrpc: str = "2.0"
    method: str
    params: dict[str, Any] = Field(default_factory=dict)
    id: int | str | None = None


class JSONRPCResponse(BaseModel):
    """JSON-RPC 2.0 response."""

    jsonrpc: str = "2.0"
    result: Any = None
    error: dict[str, Any] | None = None
    id: int | str | None = None


class PanelResult(BaseModel):
    """Result from analysing a single manga panel."""

    panel_index: int = 0
    text_detected: list[OCRResultItem] = Field(default_factory=list)
    description: str = ""
