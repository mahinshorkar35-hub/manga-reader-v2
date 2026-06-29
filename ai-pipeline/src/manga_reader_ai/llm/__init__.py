"""LLM integration — prompt templates and async client.

Ported from prompts.py (dialog prompts) with a clean async LLM client
supporting Gemini, OpenAI, and Anthropic backends.
"""

from manga_reader_ai.llm.prompts import (
    BASIC_INSTRUCTIONS,
    BASIC_PROMPT,
    BASIC_PROMPT_WITH_CONTEXT,
    CHAIN_OF_DENSITY_PROMPT,
    DRAMATIC_PROMPT,
    KEY_PAGE_IDENTIFICATION_INSTRUCTIONS,
    KEY_PANEL_IDENTIFICATION_INSTRUCTIONS,
    JSON_PARSE_PROMPT,
    JSON_PARSE_PROMPT_PANELS,
)
from manga_reader_ai.llm.client import (
    LLMClient,
    GeminiLLMClient,
    OpenAILLMClient,
    AnthropicLLMClient,
)

__all__ = [
    # Prompts
    "BASIC_PROMPT",
    "BASIC_INSTRUCTIONS",
    "BASIC_PROMPT_WITH_CONTEXT",
    "DRAMATIC_PROMPT",
    "CHAIN_OF_DENSITY_PROMPT",
    "KEY_PAGE_IDENTIFICATION_INSTRUCTIONS",
    "KEY_PANEL_IDENTIFICATION_INSTRUCTIONS",
    "JSON_PARSE_PROMPT",
    "JSON_PARSE_PROMPT_PANELS",
    # Clients
    "LLMClient",
    "GeminiLLMClient",
    "OpenAILLMClient",
    "AnthropicLLMClient",
]
