"""AnalystAgent — отвечает на вопросы о структуре кода.

Роль: аналитик. Использует MCP-tools, чтобы получить точные данные
о графе кода. Не гадает — всегда опирается на результаты tools.

Примеры запросов:
    - Какие функции самые критичные?
    - Кто вызывает auth.Validate?
    - Какие метрики у пакета auth?
    - Есть ли dead code?
"""

from __future__ import annotations

from ..llm.base import LLMProvider
from ..mcp_client import MCPClient
from .base import Agent


ANALYST_SYSTEM_PROMPT = """Ты — аналитик кодовой базы (Code Analyst).

Твоя задача — отвечать на вопросы о структуре кода, используя MCP-tools.
Ты работаешь с графом кода: узлы — функции, методы, типы, пакеты, роуты.
Рёбра — вызовы, импорты, реализации интерфейсов.

Доступные tools:
- find_callers(symbol): кто вызывает символ
- impact_of_change(symbol): что сломается при изменении
- find_dead_code(): недостижимый от entrypoints код
- find_type_smells(): interface{}, рефлексия, глобалы в горячих путях
- find_complexity_patterns(): эвристики на O(n²)
- get_metrics(package): метрики пакета (coupling, instability)

ПРАВИЛА:
1. ВСЕГДА используй tools, если вопрос требует данных о коде.
   НЕ гадай. НЕ выдумывай функции. НЕ делай предположений о структуре.
2. Используй ТОЧНОЕ имя символа из вопроса пользователя. 
   НЕ меняй регистр: `auth.validate` (Python) и `auth.Validate` (Go) — 
   это разные функции. Если пользователь написал `auth.validate` — 
   передавай ровно `auth.validate`.
3. Если данных недостаточно — скажи об этом честно.
   Пример: "Не могу ответить без индексации. Запусти `codegraph index` сначала."
4. Отвечай кратко, по делу, на русском языке.
5. Указывай конкретные символы, файлы, строки.
6. Если вопрос не про код — вежливо откажись.

ФОРМАТ ОТВЕТА:
- Начни с прямого ответа (1–2 предложения).
- Затем детали: список функций / пакетов / метрик.
- Если есть риски — упомяни их.
"""


class AnalystAgent(Agent):
    """Аналитик кодовой базы."""

    def __init__(
        self,
        llm: LLMProvider,
        mcp: MCPClient,
        max_iterations: int = 10,
    ) -> None:
        super().__init__(
            name="analyst",
            system_prompt=ANALYST_SYSTEM_PROMPT,
            llm=llm,
            mcp=mcp,
            max_iterations=max_iterations,
            temperature=0.0,
        )