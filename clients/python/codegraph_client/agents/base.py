"""Agent — базовый класс с agent loop.

Agent loop:
    1. Отправить запрос в LLM (с описанием tools).
    2. LLM решает: ответить текстом или вызвать tool.
    3. Если tool — вызвать через MCP, вернуть результат в LLM.
    4. Повторять, пока LLM не ответит текстом (или не достигнут лимит итераций).

Все агенты наследуются от этого класса, задавая свой system_prompt.
"""

from __future__ import annotations

from typing import Any

from ..llm.base import LLMProvider, LLMResponse, Message, ToolCall
from ..mcp_client import MCPClient


class AgentError(Exception):
    """Ошибка выполнения агента."""


class Agent:
    """Базовый агент: LLM + MCP-tools + agent loop."""

    def __init__(
        self,
        name: str,
        system_prompt: str,
        llm: LLMProvider,
        mcp: MCPClient,
        max_iterations: int = 10,
        temperature: float = 0.0,
    ) -> None:
        self.name = name
        self.system_prompt = system_prompt
        self.llm = llm
        self.mcp = mcp
        self.max_iterations = max_iterations
        self.temperature = temperature

    async def run(self, query: str, context: list[Message] | None = None) -> str:
        """Запустить агента на запрос.

        Args:
            query: текст запроса.
            context: опциональная история сообщений (для диалога).

        Returns:
            Финальный текстовый ответ агента.
        """
        if not self.llm.supports_tools:
            raise AgentError(
                f"LLM {self.llm.name} не поддерживает tool calling. "
                f"Агент не может работать."
            )

        tools = await self.mcp.tools_for_llm()

        messages: list[Message] = [
            Message(role="system", content=self.system_prompt),
        ]
        if context:
            messages.extend(context)
        messages.append(Message(role="user", content=query))

        for iteration in range(self.max_iterations):
            response = await self.llm.chat(
                messages,
                tools=tools,
                temperature=self.temperature,
            )

            if not response.tool_calls:
                return response.content or ""

            messages.append(
                Message(
                    role="assistant",
                    content=response.content,
                    tool_calls=response.tool_calls,
                )
            )

            for tc in response.tool_calls:
                result = await self._execute_tool(tc)
                messages.append(
                    Message(
                        role="tool",
                        tool_call_id=tc.id,
                        content=self._serialize(result),
                    )
                )

        raise AgentError(
            f"Агент {self.name} превысил лимит итераций ({self.max_iterations}). "
            f"Возможно, LLM зациклилась на вызове tools."
        )

    async def _execute_tool(self, tc: ToolCall) -> dict[str, Any]:
        """Выполнить один tool call через MCP."""
        try:
            return await self.mcp.call_tool(tc.name, tc.arguments)
        except Exception as e:
            return {"error": True, "message": f"{type(e).__name__}: {e}"}

    @staticmethod
    def _serialize(result: Any) -> str:
        """Сериализовать результат tool в строку для LLM."""
        import json
        if isinstance(result, str):
            return result
        try:
            return json.dumps(result, ensure_ascii=False, indent=2)
        except (TypeError, ValueError):
            return str(result)