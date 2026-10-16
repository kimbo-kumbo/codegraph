"""Тесты MCP-клиента.

Требуют собранного бинаря codegraph в bin/codegraph.
Запуск: pytest tests/test_mcp_client.py -v
"""

from __future__ import annotations

import pytest

from codegraph_client.mcp_client import MCPClient


@pytest.mark.asyncio
async def test_list_tools() -> None:
    """MCP-сервер отдаёт 7 tools."""
    async with MCPClient() as mcp:
        tools = await mcp.list_tools()
        assert len(tools) == 7
        names = {t.name for t in tools}
        expected = {
            "find_callers",
            "impact_of_change",
            "find_dead_code",
            "check_architecture",
            "find_type_smells",
            "find_complexity_patterns",
            "get_metrics",
        }
        assert names == expected


@pytest.mark.asyncio
async def test_call_impact_of_change() -> None:
    """impact_of_change возвращает данные из mock."""
    async with MCPClient() as mcp:
        result = await mcp.call_tool(
            "impact_of_change", {"symbol": "auth.Validate"}
        )
        assert result["symbol"]["qualified_name"] == "auth.Validate"
        assert result["total_functions"] == 2
        assert result["total_packages"] == 2
        assert "auth" in result["affected_packages"]


@pytest.mark.asyncio
async def test_call_get_metrics() -> None:
    """get_metrics возвращает список пакетов."""
    async with MCPClient() as mcp:
        result = await mcp.call_tool("get_metrics", {})
        assert "packages" in result
        assert len(result["packages"]) >= 1


@pytest.mark.asyncio
async def test_tools_for_llm() -> None:
    """tools конвертируются в формат Ollama."""
    async with MCPClient() as mcp:
        tools = await mcp.tools_for_llm()
        assert len(tools) == 7
        assert all(t["type"] == "function" for t in tools)
        names = {t["function"]["name"] for t in tools}
        assert "impact_of_change" in names