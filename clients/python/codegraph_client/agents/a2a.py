"""A2A (Agent-to-Agent) — взаимодействие между агентами.

Analyst и Advisor — два разных агента с разными ролями.
Advisor делегирует анализ Analyst'у через A2A.

Реальные A2A-серверы работают по HTTP+JSON-RPC, с Agent Cards по известному
endpoint. Здесь — in-process версия: агенты общаются через объекты Python,
но с теми же концепциями:
    - AgentCard — описание агента (имя, описание, skills).
    - A2AClient — вызов другого агента по имени метода (skill).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Awaitable, Callable

from .base import Agent


@dataclass
class AgentCard:
    """Agent Card — публичное описание агента (по спеке A2A)."""

    name: str
    description: str
    skills: list[str] = field(default_factory=list)
    endpoint: str = ""
    version: str = "0.1.0"

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "skills": self.skills,
            "endpoint": self.endpoint,
            "version": self.version,
        }


class A2ARegistry:
    """Реестр агентов: кому и как делегировать.

    Хранит Agent + AgentCard для каждого агента.
    Позволяет вызывать агента по имени (методу skill).
    """

    def __init__(self) -> None:
        self._agents: dict[str, Agent] = {}
        self._cards: dict[str, AgentCard] = {}

    def register(self, agent: Agent, card: AgentCard) -> None:
        if agent.name in self._agents:
            raise ValueError(f"Agent '{agent.name}' уже зарегистрирован")
        self._agents[agent.name] = agent
        self._cards[agent.name] = card

    def get(self, name: str) -> Agent:
        if name not in self._agents:
            raise KeyError(
                f"Agent '{name}' не зарегистрирован. "
                f"Доступные: {list(self._agents.keys())}"
            )
        return self._agents[name]

    def card(self, name: str) -> AgentCard:
        return self._cards[name]

    def cards(self) -> list[AgentCard]:
        return list(self._cards.values())

    def names(self) -> list[str]:
        return list(self._agents.keys())


class A2AClient:
    """Клиент для вызова другого агента (в in-process режиме)."""

    def __init__(self, registry: A2ARegistry, caller_name: str) -> None:
        self.registry = registry
        self.caller_name = caller_name

    async def call(self, target: str, query: str) -> str:
        """Вызвать агента `target` с запросом `query` и получить текстовый ответ."""
        if target == self.caller_name:
            raise ValueError(f"Agent '{target}' не может вызвать сам себя")
        agent = self.registry.get(target)
        return await agent.run(query)

    async def discover(self) -> list[AgentCard]:
        """Список доступных агентов (кроме себя)."""
        return [c for c in self.registry.cards() if c.name != self.caller_name]