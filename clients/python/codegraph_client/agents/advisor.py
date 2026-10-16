"""AdvisorAgent — советник по рефакторингу.

Роль: советник. Получает вопрос про рефакторинг/архитектуру, делегирует
анализ Analyst'у через A2A, на основе данных предлагает конкретные шаги.

Примеры запросов:
    - Как развязать пакеты auth и db?
    - Что можно отрефакторить в этом проекте?
    - Как убрать этот цикл зависимостей?
    - С чего начать чистку архитектуры?
"""

from __future__ import annotations

from ..llm.base import LLMProvider
from ..mcp_client import MCPClient
from .a2a import A2AClient
from .base import Agent


ADVISOR_SYSTEM_PROMPT = """Ты — советник по рефакторингу (Refactoring Advisor).

Твоя роль — предлагать конкретные шаги рефакторинга на основе данных
от аналитика. Ты не гадаешь: сначала запрашиваешь анализ через A2A,
потом предлагаешь решения.

Доступ:
1. Свои MCP-tools (те же 7, что у Analyst) — можешь вызвать напрямую,
   если нужна быстрая проверка.
2. A2A-вызов Analyst'а — для глубокого анализа. Если вопрос сложный,
   делегируй Analyst'у.

ПРАВИЛА:
1. Если вопрос требует данных о коде — сначала получи их.
   Используй tools напрямую ИЛИ делегируй Analyst'у через A2A.
2. Предлагай КОНКРЕТНЫЕ шаги, не общие слова.
   Плохо: "Улучшите архитектуру".
   Хорошо: "Вынесите интерфейс Repository в отдельный пакет repository/api,
   чтобы auth не импортировал db напрямую".
3. Указывай РИСКИ каждого изменения: что может сломаться.
4. Приоритизируй: сначала самое важное / простое / безопасное.
5. Отвечай на русском, кратко, по делу.

ФОРМАТ ОТВЕТА:
1. Диагноз (1–2 предложения): в чём проблема.
2. План (3–5 шагов): конкретные действия.
3. Риски и митигации: что может пойти не так.
"""


class AdvisorAgent(Agent):
    """Советник по рефакторингу, делегирует анализ Analyst'у через A2A."""

    def __init__(
        self,
        llm: LLMProvider,
        mcp: MCPClient,
        a2a: A2AClient,
        max_iterations: int = 12,
    ) -> None:
        super().__init__(
            name="advisor",
            system_prompt=ADVISOR_SYSTEM_PROMPT,
            llm=llm,
            mcp=mcp,
            max_iterations=max_iterations,
            temperature=0.2,
        )
        self.a2a = a2a

    async def run(self, query: str, context=None) -> str:
        """Переопределяем run, чтобы добавить A2A в контекст.

        В реальном A2A-протоколе Advisor общается с Analyst'ом через сеть.
        Здесь мы in-process: если LLM вызовет tool `call_analyst`,
        он вызовет AnalystAgent.run() через A2AClient.
        """
        return await super().run(query, context=context)

    async def delegate_to_analyst(self, query: str) -> str:
        """Прямое делегирование Analyst'у (вызывается вручную или из CLI)."""
        return await self.a2a.call("analyst", query)