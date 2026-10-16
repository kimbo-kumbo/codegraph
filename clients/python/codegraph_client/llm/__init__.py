"""LLM-слой: единый интерфейс к любой LLM."""

from .base import LLMProvider, LLMResponse, Message, ToolCall
from .factory import (
    MockProvider,
    OpenAICompatProvider,
    create_provider,
)
from .ollama import OllamaProvider

__all__ = [
    "LLMProvider",
    "LLMResponse",
    "Message",
    "ToolCall",
    "OllamaProvider",
    "OpenAICompatProvider",
    "MockProvider",
    "create_provider",
]