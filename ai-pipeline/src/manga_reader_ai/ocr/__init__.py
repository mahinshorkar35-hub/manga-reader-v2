"""Manga OCR — text extraction from manga/comic panels.

Ports logic from manga_ocr.py (EasyOCR manga OCR wrapper) with async interface.
Supports Japanese, English, and other languages.
"""

from __future__ import annotations

import asyncio
import base64
import io
import logging
from typing import Any

import numpy as np
from PIL import Image

from manga_reader_ai.config import settings
from manga_reader_ai.models.schemas import OCRBatchResult, OCRResult, OCRResultItem

logger = logging.getLogger(__name__)


class MangaOCR:
    """Thin wrapper around EasyOCR for manga panel text extraction.

    Ported from manga_ocr.py with async-first interface.
    Lazy-loads the reader on first use.
    """

    def __init__(
        self,
        languages: list[str] | None = None,
        gpu: bool = True,
    ) -> None:
        self._reader: Any = None
        self.languages = languages or settings.ocr_languages or ["ja", "en"]
        self.gpu = gpu if gpu is None else settings.ocr_gpu

    @property
    def reader(self) -> Any:
        if self._reader is None:
            self._lazy_init()
        return self._reader

    def _lazy_init(self) -> None:
        try:
            import easyocr
        except ImportError:
            raise ImportError(
                "EasyOCR is not installed.\n"
                "Install with: pip install easyocr>=1.7.0\n"
                "Or install this package with: pip install manga-reader-ai-pipeline[ocr]"
            )
        import torch

        use_gpu = self.gpu and torch.cuda.is_available()
        logger.info(
            f"[MangaOCR] Initialising EasyOCR(lang={self.languages}, gpu={use_gpu})..."
        )
        self._reader = easyocr.Reader(self.languages, gpu=use_gpu)
        logger.info(f"[MangaOCR] Reader ready ({len(self.languages)} languages)")

    async def extract_from_image(
        self, pil_image: Image.Image, detail: bool = True
    ) -> list[dict[str, Any]] | list[str]:
        """Extract text from a PIL Image (runs in thread pool)."""

        def _run() -> list[Any]:
            img_array = np.array(pil_image.convert("RGB"))
            return self.reader.readtext(img_array, detail=1)

        results = await asyncio.to_thread(_run)

        if not detail:
            return [r[1] for r in results]

        return [
            {
                "text": r[1],
                "confidence": r[2],
                "bbox": r[0],
            }
            for r in results
        ]

    async def extract_from_base64(
        self, b64_str: str, detail: bool = True
    ) -> list[dict[str, Any]] | list[str]:
        """Extract text from a base64-encoded image string."""
        img_bytes = base64.b64decode(b64_str)
        img = Image.open(io.BytesIO(img_bytes)).convert("RGB")
        return await self.extract_from_image(img, detail=detail)

    async def extract_batch(
        self, images_b64: list[str], detail: bool = True
    ) -> list[list[dict[str, Any]] | list[str]]:
        """Extract text from multiple base64 images."""
        tasks = [self.extract_from_base64(b64, detail=detail) for b64 in images_b64]
        return await asyncio.gather(*tasks)

    async def extract_to_model(
        self, images_b64: list[str]
    ) -> OCRBatchResult:
        """Extract text and return typed OCRBatchResult."""
        raw_results = await self.extract_batch(images_b64, detail=True)
        ocr_results: list[OCRResult] = []
        for idx, items in enumerate(raw_results):
            typed_items = [
                OCRResultItem(
                    text=item["text"],
                    confidence=item["confidence"],
                    bbox=item["bbox"],
                )
                for item in items
            ]
            ocr_results.append(OCRResult(image_index=idx, items=typed_items))
        return OCRBatchResult(results=ocr_results)


# Convenience alias
MangaOCREngine = MangaOCR
