"""Тесты CLI (с MockProvider, без Ollama)."""

from __future__ import annotations

import os

import pytest
from typer.testing import CliRunner

from codegraph_client.cli import app


runner = CliRunner()


@pytest.fixture(autouse=True)
def set_mock_llm(monkeypatch: pytest.MonkeyPatch) -> None:
    """Все тесты CLI работают с mock LLM и mock MCP."""
    monkeypatch.setenv("CODEGRAPH_LLM", "mock")
    monkeypatch.setenv("CODEGRAPH_MOCK", "1")


def test_version() -> None:
    result = runner.invoke(app, ["version"])
    assert result.exit_code == 0
    assert "codegraph-client" in result.stdout


def test_tools() -> None:
    result = runner.invoke(app, ["tools", "--mock"])
    assert result.exit_code == 0
    assert "impact_of_change" in result.stdout
    assert "find_callers" in result.stdout


def test_agents() -> None:
    result = runner.invoke(app, ["agents", "--mock"])
    assert result.exit_code == 0
    assert "analyst" in result.stdout
    assert "advisor" in result.stdout


def test_ask_mock() -> None:
    result = runner.invoke(app, ["ask", "--mock", "--llm", "mock", "тест"])
    assert result.exit_code == 0
    assert "mock response" in result.stdout or "analyst" in result.stdout