# CodeGraph MCP

Локальный MCP-сервер, который строит граф кода и отдаёт его LLM-агентам. 
Плюс Python-клиент с двумя агентами: **Analyst** (отвечает на вопросы о коде) 
и **Advisor** (предлагает рефакторинг).

Три языка в одной системе: **Go** — CLI и MCP-сервер, **Rust** — парсер 
и граф, **Python** — LLM-клиент. Парсит **Go + Python + Rust**, работает 
полностью локально. Может **распарсить сам себя**.

[![Go](https://img.shields.io/badge/Go-1.25+-00ADD8?logo=go)](https://go.dev)
[![Python](https://img.shields.io/badge/Python-3.11+-3776AB?logo=python)](https://python.org)
[![Rust](https://img.shields.io/badge/Rust-1.75+-000000?logo=rust)](https://rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-53%2B-brightgreen)]()

---

## Что это такое

Обычная LLM, когда её спрашивают «что сломается, если я поменяю 
`auth.Validate`?», начинает гадать. Она не видит твой проект, не знает 
кто кого вызывает, и не может пройти по цепочке зависимостей.

CodeGraph решает это: он строит граф твоего кода (функции, вызовы, 
зависимости пакетов) на **Rust через tree-sitter**, отдаёт его по 
**gRPC** в **Go MCP-сервер**, который выставляет его как **7 MCP-tools** 
для LLM-агентов на **Python**.

Ты задаёшь вопрос по-русски:

```bash
codegraph ask --agent analyst "что сломается при изменении auth.Validate?"
```

Ollama (локально, `llama3.1:8b`) решает, что нужен tool `impact_of_change`, 
вызывает его через MCP, получает данные от Rust-ядра, и формулирует:

```
При изменении функции `auth.Validate` сломается:
- auth.Logout (internal/auth/auth.go, строка 49)
- auth.Login (internal/auth/auth.go, строка 28)
- main.main (main.go, строка 11)

Риск: изменение может повлиять на работу всех трёх функций.
```

**Не выдумка. Не «обычно бывает так».** Реальные файлы, реальные строки, 
реальный граф зависимостей.

---

## Зачем это, если есть Cursor

Справедливый вопрос. Короткий ответ: **CodeGraph — не замена Cursor, 
а слой под ним.**

Cursor — это редактор с LLM. Он умеет семантический поиск, автодополнение, 
генерацию кода. Но он **не строит полный граф кода**, **не делает 
транзитивный impact analysis**, **не проверяет архитектурные правила** 
и **не работает в CI**.

CodeGraph — это **MCP-сервер**, который даёт LLM-агенту структурное 
понимание проекта. Cursor может **подключиться к нему** и получить данные, 
которых у него нет.

Как браузер и веб-сервер: не конкурируют, дополняют.

### Что делает Cursor, а что CodeGraph

| | Cursor | CodeGraph |
|---|---|---|
| Семантический поиск | ✅ | ❌ (это работа Cursor) |
| Автодополнение | ✅ | ❌ |
| Генерация кода | ✅ | ❌ |
| Транзитивный impact analysis | ❌ | ✅ |
| Архитектурные правила (YAML) | ❌ | ✅ |
| Dead code от entrypoints | ❌ | ✅ |
| Проверка в CI (SARIF, GitHub Actions) | ❌ | ✅ |
| Мультиязычный граф (Go + Python + Rust) | ❌ | ✅ |
| Self-analysis (парсит сам себя) | ❌ | ✅ |
| Работает 100% локально | ❌ (облако) | ✅ |
| LLM-agnostic (любая модель) | ❌ | ✅ |

---

## Демо

### 1. Impact analysis — реальные данные из Rust

```bash
$ codegraph index examples/go-sample
Индексация проекта: examples/go-sample

✓ Индексация завершена за 3ms
  Файлов обработано: 4
  Узлов создано:     8
  Рёбер создано:     4

$ codegraph impact --symbol auth.Validate
Символ: auth.Validate
  internal/auth/auth.go:16

Затронуто функций:  3
Затронуто пакетов:  2

Пакеты:
  - auth
  - main

Функции:
  - auth.Logout (internal/auth/auth.go:49)
  - auth.Login (internal/auth/auth.go:28)
  - main.main (main.go:11)
```

Rust-ядро распарсило Go-код через tree-sitter, построило граф через 
petgraph, нашло **все** транзитивные вызовы. Go CLI получил это по gRPC.

### 2. LLM-агент через MCP — реальный gRPC пайплайн

```bash
$ codegraph ask --agent analyst "какие метрики у пакета auth?"

╭────────────────────────────────── analyst ───────────────────────────────────╮
│ Метрики пакета auth:                                                         │
│                                                                              │
│ * Fan-in: 1                                                                  │
│ * Fan-out: 1                                                                 │
│ * Instability: 0.5                                                           │
│ * Abstractness: 0                                                            │
│ * Distance: 0.5                                                              │
│                                                                              │
│ В пакете auth 4 узла и 4 ребра.                                              │
│ Пакет имеет среднюю нестабильность (0.5).                                    │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**Что произошло под капотом:**
1. Python запустил `codegraph serve --mcp`.
2. Go MCP-сервер подключился к Rust gRPC на `localhost:50051`.
3. Ollama получила список 7 tools и решила вызвать `get_metrics({"package": "auth"})`.
4. Python вызвал MCP-tool → Go → gRPC → Rust → petgraph.
5. Реальные метрики вернулись обратно.
6. Ollama сформулировала ответ на русском.

### 3. Advisor предлагает рефакторинг — через A2A

```bash
$ codegraph ask --agent advisor "как развязать пакеты auth и db?"

╭────────────────────────────── advisor ───────────────────────────────╮
│ Диагноз: Пакеты auth и db напрямую импортируют друг друга,            │
│ что усложняет поддержку.                                              │
│                                                                       │
│ План:                                                                 │
│ 1. Вынести интерфейс Repository в пакет repository/api.               │
│ 2. Заменить прямые импорты db в auth на интерфейс.                    │
│ 3. Проверить, что db не импортирует auth.                             │
│                                                                       │
│ Риски: изменение может сломать работу системы.                        │
│ Митигация: регулярные тесты + MCP-tools для проверки зависимостей.    │
╰───────────────────────────────────────────────────────────────────────╯
```

Advisor **делегирует анализ** Analyst'у через A2A, потом формирует план.

### 4. Проверка архитектуры

```bash
$ codegraph check --rules rules/go-layers.yaml

Проверено правил: 2
Нарушений:        1

1) [error] architecture.layers
   Символ: handlers.GetUser
   internal/handlers/user.go:42:5
   handlers must not access DB directly
```

Возвращает **exit code 1** при нарушении — работает в CI.

### 5. Self-parsing — проект анализирует сам себя

```bash
$ codegraph index core/
Индексация проекта: core/

✓ Индексация завершена за 26ms
  Файлов обработано: 15
  Узлов создано:     136
  Рёбер создано:     360

$ codegraph metrics
PACKAGE              LANG         Ca     Ce        I        A        D
--------------------------------------------------------------------
codegraph-graph      rust          ...
codegraph-ir         rust          ...
codegraph-parser     rust          ...
codegraph-grpc       rust          ...
...
```

**Rust-парсер распарсил собственный код** (15 файлов, 136 узлов, 360 рёбер 
за 26 миллисекунд). Проект **анализирует сам себя** — это лучший тест 
работоспособности парсера.

### 6. Мультиязычный граф

Парсер поддерживает **три языка** — Go, Python, Rust:

```bash
$ codegraph index examples/go-sample
[codegraph-parser] parsed 4 files, 8 nodes, 4 edges from .../examples/go-sample

$ codegraph index examples/python-sample
[codegraph-parser] parsed 5 files, 7 nodes, 5 edges from .../examples/python-sample

$ codegraph index examples/rust-sample
[codegraph-parser] parsed 3 files, 7 nodes, 5 edges from .../examples/rust-sample
```

Кросс-файловые вызовы работают во всех трёх языках. Impact analysis, 
dead code, метрики — тоже.

---

## Что внутри

### 7 MCP-tools

| Tool | Что делает | Реализация |
|---|---|---|
| `find_callers(symbol)` | Кто вызывает функцию — прямые и транзитивные вызовы | ✅ Rust |
| `impact_of_change(symbol)` | Blast radius: что сломается при изменении | ✅ Rust |
| `find_dead_code()` | Код, недостижимый от entrypoints | ✅ Rust |
| `check_architecture(rules)` | Архитектурные правила из YAML | 🟡 mock (Rust — TODO) |
| `find_type_smells()` | `interface{}` в горячих путях, рефлексия, глобалы | 🟡 mock (Rust — TODO) |
| `find_complexity_patterns()` | Эвристики на O(n²): вложенные циклы, `Contains`/`sort` в цикле | ✅ Rust |
| `get_metrics(package)` | Coupling, instability, abstractness, distance | ✅ Rust |

### Агенты

**Analyst** — отвечает на вопросы о структуре кода. Всегда использует 
tools, никогда не гадает. Если данных нет — честно говорит.

**Advisor** — предлагает рефакторинг. Сначала получает анализ (свой или 
через A2A от Analyst'а), потом формирует конкретный план с рисками.

Между собой общаются через **A2A** (Agent Cards, skills, registry).

### LLM

По умолчанию — **Ollama** (локально). Поддерживаются:

- Ollama — `llama3.1:8b` (рекомендуется для tool calling), `qwen2.5-coder:7b/14b`
- DeepSeek — `deepseek-chat`
- OpenAI — `gpt-4o-mini` и выше
- Qwen — `qwen-plus` через DashScope
- Groq, Together — через OpenAI-совместимый endpoint
- Mock — для тестов без LLM

Смена модели — одна строка в env или флаг `--llm`.

---

## Архитектура

```
┌──────────────────────────────────────────────────────────┐
│  CLI (typer, Python)                                     │
│  codegraph ask / tools / agents / version                │
└────────────────────┬─────────────────────────────────────┘
                     │
┌────────────────────▼─────────────────────────────────────┐
│  Runtime                                                 │
│  ├── LLMProvider (Ollama / DeepSeek / OpenAI / Mock)     │
│  ├── MCPClient (stdio → Go MCP-сервер)                   │
│  └── Agents                                              │
│       ├── AnalystAgent                                   │
│       └── AdvisorAgent ──A2A──► AnalystAgent             │
└────────────────────┬─────────────────────────────────────┘
                     │ stdio (JSON-RPC 2.0)
┌────────────────────▼─────────────────────────────────────┐
│  Go MCP Server (codegraph serve --mcp)                   │
│  ├── 7 tools per MCP spec                                │
│  └── core.Client                                         │
│       ├── grpcClient ──► Rust core                       │
│       └── mockClient (для тестов)                        │
└────────────────────┬─────────────────────────────────────┘
                     │ gRPC (codegraph.v1)
┌────────────────────▼─────────────────────────────────────┐
│  Rust Core (codegraph-core on :50051)                    │
│  ├── ir         (типы Node, Edge, Graph, Language)       │
│  ├── parser     (tree-sitter: Go + Python + Rust)        │
│  │   └── complexity (эвристики O(n²))                    │
│  ├── graph      (petgraph: узлы, рёбра, запросы)         │
│  │   ├── queries     (find_callers, impact)              │
│  │   ├── dead_code   (достижимость от entrypoints)       │
│  │   └── metrics     (coupling, instability)             │
│  └── grpc       (tonic: реализация CodeGraph service)    │
└──────────────────────────────────────────────────────────┘
```

**Распределение языков — по их сильным сторонам:**

- **Go** — CLI, MCP-сервер, HTTP API, gRPC-клиент. Сетевая работа, конкурентность.
- **Rust** — парсинг AST (tree-sitter), граф (petgraph), gRPC-сервер (tonic). CPU-bound задача.
- **Python** — LLM-слой, агенты, A2A. Родная экосистема LLM.

---

## Быстрый старт

### Что нужно

- **Go 1.25+** — CLI и MCP-сервер.
- **Rust 1.75+** — парсер и gRPC-сервер.
- **Python 3.11+** — LLM-клиент.
- **Ollama** — локальная LLM (опционально, есть `--mock`).

### 1. Собрать Go-бинарь

```bash
git clone git@github.com:kimbo-kumbo/codegraph.git
cd codegraph
go build -o bin/codegraph ./cmd/codegraph

./bin/codegraph --help
```

### 2. Собрать Rust-ядро

```bash
cd core
cargo build --release -p codegraph-grpc
cd ..
```

**Первый раз — до 5 минут.** Бинарь: `core/target/release/codegraph-core`.

### 3. Установить Ollama и модель

```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3.1:8b
```

**Важно:** `llama3.1:8b` стабильно работает с tool calling. 
`qwen2.5-coder:7b` хорош для кода, но в старых версиях Ollama (<0.5.0) 
может возвращать tool calls текстом вместо структурированного ответа.

### 4. Установить Python-клиент

```bash
cd clients/python
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
cd ../..
```

### 5. Запустить Rust-сервер

**В отдельном терминале:**
```bash
./core/target/release/codegraph-core
# → [codegraph-core] gRPC server listening on 127.0.0.1:50051
```

### 6. Работать

**В другом терминале:**

```bash
# Индексировать проект
./bin/codegraph index examples/go-sample

# Impact analysis (реальные данные из Rust)
./bin/codegraph impact --symbol auth.Validate

# Метрики (реальные данные)
./bin/codegraph metrics

# Dead code
./bin/codegraph find-dead-code --include-exported

# Complexity patterns (O(n²))
./bin/codegraph find-complexity-patterns

# LLM-агент через MCP + gRPC
cd clients/python
source .venv/bin/activate
codegraph ask --agent analyst "какие метрики у пакета auth?"
codegraph ask --agent analyst "есть ли в проекте dead code?"
codegraph ask --agent analyst "есть ли паттерны O(n²)?"

# Mock-режим (для тестов, без Rust)
codegraph ask --mock --agent analyst "какие метрики у auth?"
```

**Альтернатива — `make`:**

```bash
make help          # список команд
make build         # собрать всё (Go + Rust)
make run-rust      # запустить Rust-сервер в фоне
make demo-real     # демо с реальным Rust
make test-all      # все тесты (Go + Python + Rust)
```

### Self-parsing

```bash
# Проанализировать сам проект
./bin/codegraph index core/
./bin/codegraph metrics
```

---

## Что умеет и что не умеет

### Умеет

- Парсить **Go + Python + Rust** код через tree-sitter.
- Строить единый мультиязычный граф с кросс-файловыми связями.
- Считать impact analysis (транзитивные вызовы с обходом).
- Находить dead code от entrypoints (main, роуты, тесты).
- Считать метрики пакетов: coupling, instability, abstractness, distance.
- Находить эвристики на O(n²): вложенные циклы, `Contains`/`sort` в цикле.
- Отвечать на вопросы о коде через LLM (Ollama/DeepSeek/OpenAI/Qwen/Groq).
- **Распарсить сам себя** (self-analysis).
- Работать в CI (exit code 1 при нарушениях).
- Работать полностью локально, без интернета.
- Использовать любую LLM через единый `LLMProvider`.
- Два агента с A2A: Analyst и Advisor.

### Не умеет (и не пытается)

- **Race conditions** — это runtime, используй `go test -race`.
- **Точный Big-O** — только эвристики. Даже CodeQL не даёт точный анализ.
- **Полный SAST** — не конкурент CodeQL/Semgrep. Используй их вместе.
- **Семантический поиск / embeddings** — это Cursor и Sourcegraph.
- **Кросс-языковые вызовы** (Go → Python через gRPC) — вне scope.
- **`check_architecture` и `find_type_smells` в Rust** — пока заглушки 
  (работают в mock-режиме). Остальные 5 tools — реально работают на Rust.

Честно указывать границы — это часть проекта. Инструмент, который 
обещает всё, обычно не делает ничего хорошо.

---

## Тесты

```bash
# Все сразу
make test-all

# По отдельности
make test-go        # Go
make test-python    # Python (12 тестов)
make test-rust      # Rust (41 тест: 9 ir + 9 graph + 23 parser)
```

**Итого — 53+ тестов**, все проходят без LLM и без Rust-сервера (через 
`MockProvider` и `mockClient`). Это позволяет разрабатывать компоненты 
независимо.

---

## О проекте

Это pet-проект. Сделан, чтобы показать:

- **Многоязычная архитектура** — Go + Rust + Python в одной системе, 
  каждый на своём месте. gRPC между Go и Rust, MCP между Go и Python.
- **Три парсера** — Go, Python, Rust через tree-sitter. Единый IR, 
  единый граф, единые запросы.
- **MCP** — реализация сервера по официальной спеке + собственный клиент.
- **A2A** — взаимодействие агентов через Agent Cards.
- **LLM-agnostic** — единый интерфейс к любой модели, Ollama по умолчанию.
- **Приватность** — 100% локально, никакие данные не уходят в облако.
- **Self-analysis** — проект умеет анализировать сам себя.
- **Инженерная честность** — явные границы применимости, никаких обещаний 
  «революции».

---

## Лицензия

Apache 2.0 — см. [LICENSE](LICENSE).

---