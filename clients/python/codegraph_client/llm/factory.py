"""Фабрика LLMProvider + MockProvider для тестов."""

from __future__ import annotations
import os
from typing import Any, AsyncIterator
from .base import LLMProvider, LLMResponse, Message, ToolCall
from .ollama import OllamaProvider

class OpenAICompatProvider(LLMProvider):
    """Работает с любым OpenAI-совместимым API."""

    def __init__(
        self,
        model: str,
        api_key: str,
        base_url: str = "https://api.openai.com/v1",
    ) -> None:
        from openai import AsyncOpenAI

        self.client = AsyncOpenAI(api_key=api_key, base_url=base_url)
        self.model = model

    @property
    def supports_tools(self) -> bool:
        return True

    @property
    def name(self) -> str:
        return f"openai-compat/{self.model}"

    async def chat(
        self,
        messages: list[Message],
        tools: list[dict[str, Any]] | None = None,
        temperature: float = 0.0,
    ) -> LLMResponse:
        resp = await self.client.chat.completions.create(
            model=self.model,
            messages=[self._to_dict(m) for m in messages],
            tools=tools or None,
            temperature=temperature,
        )
        choice = resp.choices[0]
        tool_calls = [
            ToolCall(
                id=tc.id,
                name=tc.function.name,
                arguments=self._parse_args(tc.function.arguments),
            )
            for tc in (choice.message.tool_calls or [])
        ]
        return LLMResponse(
            content=choice.message.content,
            tool_calls=tool_calls,
            finish_reason=choice.finish_reason or "stop",
        )

    async def stream(
        self,
        messages: list[Message],
        tools: list[dict[str, Any]] | None = None,
    ) -> AsyncIterator[str]:
        stream = await self.client.chat.completions.create(
            model=self.model,
            messages=[self._to_dict(m) for m in messages],
            stream=True,
        )
        async for chunk in stream:
            if chunk.choices and chunk.choices[0].delta.content:
                yield chunk.choices[0].delta.content

    def _to_dict(self, m: Message) -> dict[str, Any]:
        d: dict[str, Any] = {"role": m.role}
        if m.content is not None:
            d["content"] = m.content
        if m.tool_calls:
            d["tool_calls"] = [
                {
                    "id": tc.id,
                    "type": "function",
                    "function": {"name": tc.name, "arguments": _json(tc.arguments)},
                }
                for tc in m.tool_calls
            ]
        if m.tool_call_id:
            d["tool_call_id"] = m.tool_call_id
        return d

    def _parse_args(self, raw: str) -> dict[str, Any]:
        import json
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {}


def _json(v: Any) -> str:
    import json
    return json.dumps(v, ensure_ascii=False)

class MockProvider(LLMProvider):
    """Mock LLM для тестов. Возвращает заранее заданные ответы."""

    def __init__(self, responses: list[LLMResponse] | None = None) -> None:
        self._responses = responses or []
        self._calls: list[list[Message]] = []

    @property
    def supports_tools(self) -> bool:
        return True

    @property
    def name(self) -> str:
        return "mock"

    async def chat(
        self,
        messages: list[Message],
        tools: list[dict[str, Any]] | None = None,
        temperature: float = 0.0,
    ) -> LLMResponse:
        self._calls.append(messages)
        if self._responses:
            return self._responses.pop(0)
        return LLMResponse(content="mock response", tool_calls=[])

    async def stream(
        self,
        messages: list[Message],
        tools: list[dict[str, Any]] | None = None,
    ) -> AsyncIterator[str]:
        yield "mock stream"

    @property
    def calls(self) -> list[list[Message]]:
        """Записанные вызовы для assertions в тестах."""
        return self._calls

def create_provider(name: str | None = None) -> LLMProvider:
    """Создать провайдера по имени.

    Args:
        name: "ollama" (default), "deepseek", "openai", "qwen", "mock".

    Env:
        CODEGRAPH_LLM — имя провайдера.
        OLLAMA_MODEL, OLLAMA_URL — для Ollama.
        DEEPSEEK_API_KEY, DEEPSEEK_MODEL, DEEPSEEK_BASE_URL — для DeepSeek.
        OPENAI_API_KEY, OPENAI_MODEL, OPENAI_BASE_URL — для OpenAI.
    """
    name = name or os.getenv("CODEGRAPH_LLM", "ollama")

    if name == "mock":
        return MockProvider()

    if name == "ollama":
        return OllamaProvider(
            model=os.getenv("OLLAMA_MODEL", "llama3.1:8b"),
            base_url=os.getenv("OLLAMA_URL", "http://localhost:11434"),
        )

    if name in ("deepseek", "openai", "qwen", "groq", "together"):
        prefix = name.upper()
        api_key = os.getenv(f"{prefix}_API_KEY")
        if not api_key:
            raise ValueError(f"{prefix}_API_KEY is required for provider '{name}'")
        return OpenAICompatProvider(
            model=os.getenv(f"{prefix}_MODEL", _default_model(name)),
            api_key=api_key,
            base_url=os.getenv(f"{prefix}_BASE_URL", _default_base_url(name)),
        )

    raise ValueError(f"Unknown LLM provider: {name}")


def _default_model(name: str) -> str:
    return {
        "deepseek": "deepseek-chat",
        "openai": "gpt-4o-mini",
        "qwen": "qwen-plus",
        "groq": "llama-3.1-70b-versatile",
        "together": "meta-llama/Llama-3.1-70B-Instruct-Turbo",
    }[name]


def _default_base_url(name: str) -> str:
    return {
        "deepseek": "https://api.deepseek.com/v1",
        "openai": "https://api.openai.com/v1",
        "qwen": "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "groq": "https://api.groq.com/openai/v1",
        "together": "https://api.together.xyz/v1",
    }[name]