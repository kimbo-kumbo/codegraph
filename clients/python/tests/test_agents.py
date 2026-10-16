"""Тесты агентов с MockProvider (без Ollama)."""

from __future__ import annotations

import pytest

from codegraph_client.agents import (
    A2AClient,
    A2ARegistry,
    AgentCard,
    AnalystAgent,
    AdvisorAgent,
)
from codegraph_client.llm.base import LLMResponse, Message, ToolCall
from codegraph_client.llm.factory import MockProvider
from codegraph_client.mcp_client import MCPClient


@pytest.mark.asyncio
async def test_analyst_with_mock_llm() -> None:
    """Analyst с MockProvider: LLM просит tool, получает данные, отвечает."""
    mock_llm = MockProvider(responses=[
        LLMResponse(
            content=None,
            tool_calls=[
                ToolCall(
                    id="call_1",
                    name="impact_of_change",
                    arguments={"symbol": "auth.Validate"},
                ),
            ],
            finish_reason="tool_calls",
        ),
        LLMResponse(
            content="Изменение auth.Validate затронет 2 функции в 2 пакетах.",
            tool_calls=[],
            finish_reason="stop",
        ),
    ])

    async with MCPClient() as mcp:
        agent = AnalystAgent(llm=mock_llm, mcp=mcp)
        answer = await agent.run("Что сломается при изменении auth.Validate?")

    assert "auth.Validate" in answer
    assert "2" in answer
    assert len(mock_llm.calls) == 2


@pytest.mark.asyncio
async def test_analyst_rejects_llm_without_tools() -> None:
    """Analyst ругается, если LLM не поддерживает tools."""

    class NoToolsProvider(MockProvider):
        @property
        def supports_tools(self) -> bool:
            return False

    async with MCPClient() as mcp:
        agent = AnalystAgent(llm=NoToolsProvider(), mcp=mcp)
        with pytest.raises(Exception, match="не поддерживает tool calling"):
            await agent.run("тест")


@pytest.mark.asyncio
async def test_a2a_registry_and_delegation() -> None:
    """Advisor делегирует Analyst'у через A2A."""
    mock_llm_analyst = MockProvider(responses=[
        LLMResponse(content="Метрики auth: instability=0.625", tool_calls=[]),
    ])
    mock_llm_advisor = MockProvider(responses=[
        LLMResponse(content="План: развязать через интерфейс", tool_calls=[]),
    ])

    async with MCPClient() as mcp:
        analyst = AnalystAgent(llm=mock_llm_analyst, mcp=mcp)
        registry = A2ARegistry()
        registry.register(
            analyst,
            AgentCard(
                name="analyst",
                description="Аналитик кодовой базы",
                skills=["find_callers", "impact_of_change"],
            ),
        )

        advisor = AdvisorAgent(
            llm=mock_llm_advisor,
            mcp=mcp,
            a2a=A2AClient(registry, caller_name="advisor"),
        )
        registry.register(
            advisor,
            AgentCard(
                name="advisor",
                description="Советник по рефакторингу",
                skills=["suggest_refactoring"],
            ),
        )

        result = await advisor.delegate_to_analyst("метрики auth?")
        assert "instability" in result or "auth" in result


@pytest.mark.asyncio
async def test_agent_card_serialization() -> None:
    """AgentCard сериализуется в JSON-совместимый dict."""
    card = AgentCard(
        name="analyst",
        description="Аналитик",
        skills=["find_callers"],
        endpoint="http://localhost:8001/a2a",
    )
    d = card.to_dict()
    assert d["name"] == "analyst"
    assert d["skills"] == ["find_callers"]
    assert d["endpoint"].startswith("http://")