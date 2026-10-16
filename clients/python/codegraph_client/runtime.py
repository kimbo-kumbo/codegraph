"""Runtime — фабрика, которая собирает всё вместе.

Создаёт:
    - Config (пути, режим mock/grpc)
    - LLMProvider (ollama, deepseek, openai, mock)
    - MCPClient (запускает codegraph serve --mcp)
    - Агентов (Analyst + Advisor)
    - A2A-registry

Использование:
    async with Runtime(llm_name="ollama") as rt:
        answer = await rt.run_agent("analyst", "какие метрики у auth?")
"""

from __future__ import annotations

from contextlib import AsyncExitStack
from typing import Any

from .agents import (
    A2AClient,
    A2ARegistry,
    AdvisorAgent,
    AgentCard,
    AnalystAgent,
)
from .config import Config, load_config
from .llm import LLMProvider, create_provider
from .mcp_client import MCPClient


ANALYST_CARD = AgentCard(
    name="analyst",
    description="Анализирует структуру кодовой базы через CodeGraph MCP",
    skills=[
        "find_callers",
        "impact_of_change",
        "find_dead_code",
        "find_type_smells",
        "find_complexity_patterns",
        "get_metrics",
    ],
    endpoint="in-process://analyst",
)

ADVISOR_CARD = AgentCard(
    name="advisor",
    description="Предлагает рефакторинг на основе анализа Analyst'а",
    skills=["suggest_refactoring", "prioritize_changes"],
    endpoint="in-process://advisor",
)


class Runtime:
    """Среда выполнения: LLM + MCP + агенты."""

    def __init__(
        self,
        llm_name: str | None = None,
        config: Config | None = None,
    ) -> None:
        self.config = config or load_config()
        self._llm_name = llm_name
        self._exit_stack: AsyncExitStack | None = None
        self._mcp: MCPClient | None = None
        self._llm: LLMProvider | None = None
        self._registry: A2ARegistry | None = None
        self._agents: dict[str, Any] = {}

    async def __aenter__(self) -> "Runtime":
        self._exit_stack = AsyncExitStack()
        try:
            self._llm = create_provider(self._llm_name)
            self._mcp = await self._exit_stack.enter_async_context(MCPClient(self.config))
            self._registry = A2ARegistry()
            analyst = AnalystAgent(llm=self._llm, mcp=self._mcp)
            self._registry.register(analyst, ANALYST_CARD)

            advisor = AdvisorAgent(
                llm=self._llm,
                mcp=self._mcp,
                a2a=A2AClient(self._registry, caller_name="advisor"),
            )
            self._registry.register(advisor, ADVISOR_CARD)

            self._agents = {"analyst": analyst, "advisor": advisor}
        except Exception:
            await self._exit_stack.aclose()
            raise
        return self

    async def __aexit__(self, *args: Any) -> None:
        if self._exit_stack is not None:
            await self._exit_stack.aclose()
            self._exit_stack = None
            self._mcp = None
            self._registry = None
            self._agents = {}

    @property
    def llm(self) -> LLMProvider:
        assert self._llm is not None, "Runtime not entered"
        return self._llm

    @property
    def mcp(self) -> MCPClient:
        assert self._mcp is not None, "Runtime not entered"
        return self._mcp

    @property
    def registry(self) -> A2ARegistry:
        assert self._registry is not None, "Runtime not entered"
        return self._registry

    async def run_agent(self, agent_name: str, query: str) -> str:
        """Запустить агента с запросом, вернуть текстовый ответ."""
        if agent_name not in self._agents:
            raise ValueError(
                f"Неизвестный агент: {agent_name}. "
                f"Доступные: {list(self._agents.keys())}"
            )
        agent = self._agents[agent_name]
        return await agent.run(query)

    async def list_tools(self) -> list[dict[str, Any]]:
        """Список tools в формате LLM."""
        return await self.mcp.tools_for_llm()

    def list_agents(self) -> list[AgentCard]:
        """Список Agent Cards."""
        return self.registry.cards()