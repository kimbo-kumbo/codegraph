"""LLMProvider — единый интерфейс для любой LLM.

Реализации:
    - OllamaProvider — локально, приватно (по умолчанию).
    - OpenAICompatProvider — DeepSeek, Qwen, OpenAI, Together, Groq.

Контракт (нельзя ломать):
    - chat(messages, tools, temperature) -> LLMResponse
    - stream(messages, tools) -> AsyncIterator[str]
    - supports_tools -> bool

Message/LLMResponse/ToolCall — общие типы для всех провайдеров.
"""

from __future__ import annotations
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, AsyncIterator, Literal


Role = Literal["system", "user", "assistant", "tool"]

@dataclass
class Message:
    """Сообщение в диалоге."""

    role: Role
    content: str | None = None
    tool_calls: list["ToolCall"] = field(default_factory=list)
    tool_call_id: str | None = None


@dataclass
class ToolCall:
    """Запрос LLM на вызов инструмента."""

    id: str
    name: str
    arguments: dict[str, Any]


@dataclass
class LLMResponse:
    """Ответ LLM."""

    content: str | None
    tool_calls: list[ToolCall] = field(default_factory=list)
    finish_reason: str = "stop"


class LLMProvider(ABC):
    """Единый интерфейс для любой LLM."""

    @abstractmethod
    async def chat(
        self,
        messages: list[Message],
        tools: list[dict[str, Any]] | None = None,
        temperature: float = 0.0,
    ) -> LLMResponse:
        """Отправить сообщения, получить ответ (возможно с tool_calls)."""

    @abstractmethod
    async def stream(
        self,
        messages: list[Message],
        tools: list[dict[str, Any]] | None = None,
    ) -> AsyncIterator[str]:
        """Стриминг ответа (для CLI)."""

    @property
    @abstractmethod
    def supports_tools(self) -> bool:
        """Поддерживает ли провайдер tool calling."""

    @property
    def name(self) -> str:
        """Имя провайдера для логов."""
        return self.__class__.__name__