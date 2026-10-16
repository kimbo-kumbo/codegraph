"""MCP-клиент для codegraph serve --mcp.

Запускает codegraph как дочерний процесс, общается через stdio (stdin/stdout)
по спецификации MCP (JSON-RPC 2.0).

Использование:
    async with MCPClient() as mcp:
        tools = await mcp.list_tools()       # 7 tools
        result = await mcp.call_tool("impact_of_change", {"symbol": "auth.Validate"})
"""

from __future__ import annotations

import json
from contextlib import AsyncExitStack
from dataclasses import dataclass
from typing import Any

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

from .config import Config, load_config


@dataclass
class ToolInfo:
    """Информация о tool из MCP."""

    name: str
    description: str
    input_schema: dict[str, Any]

    def to_ollama_format(self) -> dict[str, Any]:
        """Конвертировать в формат Ollama/OpenAI tools."""
        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.input_schema,
            },
        }


class MCPClient:
    """Async context manager для MCP-сессии с codegraph.

    Пример:
        async with MCPClient() as mcp:
            tools = await mcp.list_tools()
            result = await mcp.call_tool("find_callers", {"symbol": "auth.Validate"})
    """

    def __init__(self, config: Config | None = None) -> None:
        self.config = config or load_config()
        self._exit_stack: AsyncExitStack | None = None
        self._session: ClientSession | None = None
        self._tools: list[ToolInfo] | None = None

    async def __aenter__(self) -> "MCPClient":
        cmd, args = self.config.server_command()

        server_params = StdioServerParameters(
            command=cmd,
            args=args,
            env=None,
        )

        self._exit_stack = AsyncExitStack()
        try:
            read, write = await self._exit_stack.enter_async_context(
                stdio_client(server_params)
            )
            self._session = await self._exit_stack.enter_async_context(
                ClientSession(read, write)
            )
            await self._session.initialize()
        except Exception:
            await self._exit_stack.aclose()
            raise
        return self

    async def __aexit__(self, *args: Any) -> None:
        if self._exit_stack is not None:
            await self._exit_stack.aclose()
            self._exit_stack = None
            self._session = None
            self._tools = None

    async def list_tools(self) -> list[ToolInfo]:
        """Получить список tools (кэшируется)."""
        if self._tools is not None:
            return self._tools
        assert self._session is not None, "MCPClient not entered"

        response = await self._session.list_tools()
        self._tools = [
            ToolInfo(
                name=t.name,
                description=t.description or "",
                input_schema=dict(t.input_schema),
            )
            for t in response.tools
        ]
        return self._tools

    async def tools_for_llm(self) -> list[dict[str, Any]]:
        """Список tools в формате, понятном Ollama/OpenAI."""
        tools = await self.list_tools()
        return [t.to_ollama_format() for t in tools]

    async def call_tool(
        self,
        name: str,
        arguments: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Вызвать tool, вернуть распарсенный JSON.

        MCP возвращает CallToolResult с Content (список). Мы берём первый
        TextContent, парсим как JSON.
        """
        assert self._session is not None, "MCPClient not entered"

        response = await self._session.call_tool(name, arguments or {})

        if response.is_error:
            return {
                "error": True,
                "message": self._extract_text(response.content),
            }

        for item in response.content:
            if getattr(item, "type", None) == "text":
                text = item.text
                try:
                    return json.loads(text)
                except json.JSONDecodeError:
                    return {"raw": text}
        return {}

    @staticmethod
    def _extract_text(content: list[Any]) -> str:
        parts: list[str] = []
        for item in content:
            if getattr(item, "type", None) == "text":
                parts.append(item.text)
        return "\n".join(parts)