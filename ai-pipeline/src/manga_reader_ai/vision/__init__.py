"""Gemini Vision analyser — manga page and panel analysis.

Ports logic from vision_analysis.py, switching from OpenAI GPT-4o to Google Gemini.
"""

from __future__ import annotations

import json
import logging
from typing import Any

from manga_reader_ai.models.schemas import (
    AnalysisRequest,
    AnalysisResult,
    ImageData,
    ImportantPage,
)
from manga_reader_ai.config import settings

logger = logging.getLogger(__name__)

# ── Prompt templates (ported from prompts.py) ──────────────────────────

BASIC_PROMPT = """
I am giving you a sequence of pages directly out of a manga.
Please write me a SHORT, CONCISE summary of all the pages in a story-telling tone (MAXIMUM 200 WORDS).
I don't want you to invent new things, just summarize what is happening in the pages provided.

Your final summary should stick to the plot without over embellishing. The summary you write should be shorter than a minute of reading time.

REQUIRED: Please include in-line citations to the relevant image you are referring to in the format of `[^{image_index}]`.
The `image_index` is the index of the image in the sequence of pages you are summarizing, NOT the page number written on the image.
People will DIE if you cite the incorrect `image_index`.

If you'd like to provide multiple `image_index` citations next to each other, simply write them all in sequence, like this: `[^{image_index1}][^{image_index2}][^{image_index3}]`.

SUPER IMPORTANT: Sprinkle in direct quotes from particularly intense parts in your storytelling.
REQUIRED: Every direct quote MUST have an `image_index` citation.

REQUIRED: In-line image_index citations MUST be included in the same sentence as the text they are referencing. NO EXCEPTIONS.
"""

BASIC_INSTRUCTIONS = """Your job is to summarize the sequence of pages out of the manga in a compelling, storytelling tone. Don't be long-winded and stick to the plot.
The summary you write should be shorter than a minute of reading time (MAXIMUM 200 WORDS).
Please strive to sprinkle in some direct quotes from particularly intense parts to enhance your storytelling.
REQUIRED: You MUST include in-line citations to the relevant image you are referring to in the format of `[^{image_index}]`.
IMPORTANT: The `image_index` is the index of the image in the sequence of pages you are summarizing, NOT the page number written on the image.
People will DIE if you cite the incorrect `image_index`.

If you'd like to provide multiple `image_index` citations next to each other, simply write them all in sequence, like this: `[^{image_index1}][^{image_index2}][^{image_index3}]`.

SUPER IMPORTANT: Please strive to sprinkle in some direct quotes from characters during particularly intense parts to enhance your storytelling.
REQUIRED: Every direct quote MUST have an `image_index` citation.

REQUIRED: In-line image_index citations MUST be included in the same sentence as the text they are referencing. NO EXCEPTIONS."""

KEY_PAGE_IDENTIFICATION_INSTRUCTIONS = """
You are given 20 pages from a manga (indexed 0-19, in order). Your job is to detect if any of the pages are
1. A character profile page, detailing an introduction of the key characters in the manga
2. A chapter start page, implying the start of a new chapter

If any of the pages given to you contain one of those two things, please return the index of the page and the type of page it is ("profile" or "chapter").
There can be multiple profile pages and chapter pages.

Your response must be in the following format:
{"important_pages": Array<{"image_index": int 0-19, "type": "profile" | "chapter"}>}

Example:
```
{
    "important_pages": [
        {"image_index": 0, "type": "profile"},
        {"image_index": 17, "type": "chapter"}
    ]
}
```

If none of the pages contain a character profile or chapter start, return an empty array:
```
{
    "important_pages": []
}
```

Please respond with nothing else other than a properly formatted JSON object. If you fail to do so, people will die.
"""

KEY_PANEL_IDENTIFICATION_INSTRUCTIONS = """
You are given a sequence of panel images from a manga (indexed starting from 0). Your job is to identify which panels are the most relevant to the text provided.

Your response must be in the following format:
{"important_panels": Array<int>}

Example:
```
{
    "important_panels": [0, 2, 5]
}
```

Each number in the array represents the index of the panel in the sequence of panels you are given.
Always return at least one panel index. Limit your selections to the most relevant panels to the text provided.

Please respond with nothing else other than a properly formatted JSON object. If you fail to do so, people will die.
```
"""

JSON_PARSE_PROMPT = """
You are a JSON parser. Return a properly formatted json object based on the input from the user.
Your response must be in the following format:
{"important_pages": Array<{"image_index": int 0-19, "type": "profile" | "chapter"}>}

Examples of valid responses:
```
{
    "important_pages": [
        {"image_index": 0, "type": "profile"},
        {"image_index": 17, "type": "chapter"}
    ]
}
```

```
{
    "important_pages": []
}
```
"""

JSON_PARSE_PROMPT_PANELS = """
You are a JSON parser. Return a properly formatted json object based on the input from the user.
Your response must be in the following format:
{"important_panels": Array<int>}

Examples of valid responses:
```
{
    "important_panels": [
        0, 4,
    ]
}
```

```
{
    "important_panels": []
}
```
"""


# ── Gemini Client ───────────────────────────────────────────────────────


class GeminiAnalyzer:
    """Analyses manga pages using Google Gemini vision models.

    Ports logic from vision_analysis.py's GPT-4o calls to Gemini.
    """

    def __init__(self, api_key: str | None = None, model: str | None = None) -> None:
        self.api_key = api_key or settings.gemini_api_key
        self.model_name = model or settings.gemini_model
        self._client: Any = None

    @property
    def client(self) -> Any:
        if self._client is None:
            from google import genai

            self._client = genai.Client(api_key=self.api_key)
        return self._client

    # ── helpers ──────────────────────────────────────────────────────

    def _build_image_parts(self, images: list[ImageData]) -> list[Any]:
        """Convert ImageData list to Gemini inline data parts."""
        from google.genai import types

        parts: list[Any] = []
        for img in images:
            parts.append(
                types.Part.from_bytes(
                    data=img.data.encode("ascii") if isinstance(img.data, str) else img.data,
                    mime_type=img.mime_type,
                )
            )
        return parts

    def _build_text_image_contents(
        self,
        text: str,
        images: list[ImageData],
        system_instruction: str | None = None,
    ) -> list[dict[str, Any]]:
        """Build contents list for Gemini: text + images."""
        from google.genai import types

        parts: list[Any] = [types.Part.from_text(text=text)]
        parts.extend(self._build_image_parts(images))
        return [{"role": "user", "parts": parts}]

    # ── public methods ───────────────────────────────────────────────

    async def analyze_images(
        self,
        pages: list[ImageData],
        character_profiles: list[ImageData] | None = None,
        prompt: str = "",
        instructions: str = "",
        detail: str = "low",
    ) -> AnalysisResult:
        """Analyse manga pages and produce a summary with citations.

        Ports vision_analysis.analyze_images_with_gpt4_vision.
        """
        from google.genai import types

        actual_prompt = prompt or BASIC_PROMPT
        actual_instructions = instructions or BASIC_INSTRUCTIONS

        parts: list[Any] = []

        # Character profiles first, if any
        if character_profiles:
            parts.append(
                types.Part.from_text(
                    text="Here are some character profile pages, for your reference:"
                )
            )
            parts.extend(self._build_image_parts(character_profiles))

        # Pages
        parts.append(types.Part.from_text(text=actual_prompt))
        parts.extend(self._build_image_parts(pages))

        contents = [{"role": "user", "parts": parts}]

        response = await self._async_generate(
            contents=contents,
            system_instruction=actual_instructions,
            config={"max_output_tokens": settings.llm_max_tokens},
        )

        text = response.text if hasattr(response, "text") else str(response)

        return AnalysisResult(
            text=text,
            total_tokens=getattr(response, "usage_metadata", None)
            and getattr(response.usage_metadata, "total_token_count", 0)
            or 0,
            raw_response=text,
        )

    async def detect_important_pages(
        self,
        pages: list[ImageData],
        profile_reference: list[ImageData] | None = None,
        chapter_reference: list[ImageData] | None = None,
        prompt: str = "",
        instructions: str = "",
    ) -> AnalysisResult:
        """Detect character profile / chapter start pages.

        Ports vision_analysis.detect_important_pages.
        """
        from google.genai import types

        actual_prompt = prompt or "Identify important pages from the provided manga pages."
        actual_instructions = instructions or KEY_PAGE_IDENTIFICATION_INSTRUCTIONS

        parts: list[Any] = []

        if profile_reference:
            parts.append(
                types.Part.from_text(
                    text="Here are some character profile pages, for your reference:"
                )
            )
            parts.extend(self._build_image_parts(profile_reference))

        if chapter_reference:
            parts.append(
                types.Part.from_text(
                    text="Here are some chapter start pages, for your reference:"
                )
            )
            parts.extend(self._build_image_parts(chapter_reference))

        parts.append(types.Part.from_text(text=actual_prompt))
        parts.extend(self._build_image_parts(pages))

        contents = [{"role": "user", "parts": parts}]

        response = await self._async_generate(
            contents=contents,
            system_instruction=actual_instructions,
            config={"max_output_tokens": 2048},
        )

        text = response.text if hasattr(response, "text") else str(response)
        tokens = (
            getattr(response, "usage_metadata", None)
            and getattr(response.usage_metadata, "total_token_count", 0)
            or 0
        )

        # Parse JSON from response
        important_pages: list[ImportantPage] = []
        try:
            parsed = json.loads(text)
            for item in parsed.get("important_pages", []):
                important_pages.append(
                    ImportantPage(image_index=item["image_index"], type=item["type"])
                )
        except (json.JSONDecodeError, KeyError, TypeError):
            logger.warning("Failed to parse important_pages JSON, retrying with LLM fix...")
            try:
                fixed = await self._fix_json(text, JSON_PARSE_PROMPT)
                parsed = json.loads(fixed)
                for item in parsed.get("important_pages", []):
                    important_pages.append(
                        ImportantPage(image_index=item["image_index"], type=item["type"])
                    )
            except (json.JSONDecodeError, KeyError, TypeError) as e:
                logger.error(f"JSON fix also failed: {e}")

        return AnalysisResult(
            text=text,
            total_tokens=tokens,
            important_pages=important_pages,
            raw_response=text,
        )

    async def get_important_panels(
        self,
        panels: list[ImageData],
        prompt: str = "",
        instructions: str = "",
        character_profiles: list[ImageData] | None = None,
    ) -> AnalysisResult:
        """Identify which panels are most relevant to the given text.

        Ports vision_analysis.get_important_panels.
        """
        from google.genai import types

        actual_instructions = instructions or KEY_PANEL_IDENTIFICATION_INSTRUCTIONS

        parts: list[Any] = []
        if character_profiles:
            parts.append(
                types.Part.from_text(
                    text="Here are some character profile pages, for your reference:"
                )
            )
            parts.extend(self._build_image_parts(character_profiles))

        parts.append(types.Part.from_text(text=prompt))
        parts.extend(self._build_image_parts(panels))

        contents = [{"role": "user", "parts": parts}]

        response = await self._async_generate(
            contents=contents,
            system_instruction=actual_instructions,
            config={"max_output_tokens": 2048},
        )

        text = response.text if hasattr(response, "text") else str(response)
        tokens = (
            getattr(response, "usage_metadata", None)
            and getattr(response.usage_metadata, "total_token_count", 0)
            or 0
        )

        important_panels: list[int] = []
        try:
            parsed = json.loads(text)
            important_panels = parsed.get("important_panels", [])
        except (json.JSONDecodeError, KeyError, TypeError):
            logger.warning("Failed to parse important_panels JSON, retrying...")
            try:
                fixed = await self._fix_json(text, JSON_PARSE_PROMPT_PANELS)
                parsed = json.loads(fixed)
                important_panels = parsed.get("important_panels", [])
            except (json.JSONDecodeError, KeyError, TypeError) as e:
                logger.error(f"JSON fix also failed: {e}")

        return AnalysisResult(
            text=text,
            total_tokens=tokens,
            important_panels=important_panels,
            raw_response=text,
        )

    # ── internals ────────────────────────────────────────────────────

    async def _async_generate(
        self,
        contents: list[dict[str, Any]],
        system_instruction: str | None = None,
        config: dict[str, Any] | None = None,
    ) -> Any:
        """Run Gemini generation asynchronously via the sync client in a thread."""
        import asyncio
        from functools import partial

        cfg = config or {}
        fn = partial(
            self.client.models.generate_content,
            model=self.model_name,
            contents=contents,
            config=cfg,
        )
        if system_instruction:
            fn = partial(
                self.client.models.generate_content,
                model=self.model_name,
                contents=contents,
                config={**cfg, "system_instruction": system_instruction},
            )
        return await asyncio.to_thread(fn)

    async def _fix_json(self, malformed: str, fix_prompt: str) -> str:
        """Use the LLM to fix a malformed JSON response."""
        import asyncio
        from functools import partial

        from google.genai import types

        fn = partial(
            self.client.models.generate_content,
            model=self.model_name,
            contents=[
                {"role": "user", "parts": [types.Part.from_text(text=malformed)]}
            ],
            config={
                "system_instruction": fix_prompt,
                "max_output_tokens": 2048,
            },
        )
        response = await asyncio.to_thread(fn)
        return response.text if hasattr(response, "text") else str(response)
