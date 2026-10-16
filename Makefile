# CodeGraph MCP — единая точка сборки.
#
# Использование:
#     make help       — список команд
#     make build      — собрать всё (Go + Rust)
#     make test-all   — все тесты (Go + Python + Rust)
#     make demo       — демо на mock-данных
#     make demo-real  — демо с реальным Rust-сервером
#     make run-rust   — запустить Rust-сервер в фоне
#     make clean      — удалить артефакты

.PHONY: help build build-go build-rust test test-all test-go test-python test-rust \
        demo demo-llm demo-advisor demo-real run-rust stop-rust \
        install-python install-rust lint fmt clean

.DEFAULT_GOAL := help

# Пути.
GO_BIN      := bin/codegraph
GO_CMD      := ./cmd/codegraph
RUST_BIN    := core/target/release/codegraph-core
PYTHON_DIR  := clients/python
PYTHON_VENV := $(PYTHON_DIR)/.venv
PYTHON_ACT  := source $(PYTHON_VENV)/bin/activate
EXAMPLE     := examples/go-sample

help: ## Показать список команд
	@echo "CodeGraph MCP — доступные команды:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""

build: build-go build-rust ## Собрать всё: Go-бинарь + Rust-ядро

build-go: ## Собрать Go-бинарь codegraph
	@echo "→ Сборка Go-бинаря..."
	@go build -o $(GO_BIN) $(GO_CMD)
	@echo "✓ Готово: $(GO_BIN)"

build-rust: ## Собрать Rust-ядро (gRPC-сервер)
	@echo "→ Сборка Rust-ядра..."
	@cd core && cargo build --release -p codegraph-grpc
	@echo "✓ Готово: $(RUST_BIN)"

run-rust: build-rust ## Запустить Rust gRPC-сервер в фоне
	@echo "→ Запуск Rust-сервера в фоне..."
	@if pgrep -f codegraph-core > /dev/null; then \
		echo "✓ Сервер уже запущен"; \
	else \
		nohup $(RUST_BIN) > /tmp/codegraph-core.log 2>&1 & \
		sleep 1; \
		echo "✓ Сервер запущен. PID: $$(pgrep -f codegraph-core). Лог: /tmp/codegraph-core.log"; \
	fi

stop-rust: ## Остановить Rust gRPC-сервер
	@if pgrep -f codegraph-core > /dev/null; then \
		pkill -f codegraph-core || true; \
		echo "✓ Сервер остановлен"; \
	else \
		echo "Сервер не запущен"; \
	fi

test-all: test-go test-python test-rust ## Все тесты (Go + Python + Rust)
	@echo ""
	@echo "✓ Все тесты прошли"

test: test-all ## Алиас для test-all

test-go: ## Тесты на Go
	@echo "→ Go tests..."
	@go test ./... -count=1
	@echo "✓ Go OK"

test-python: ## Тесты на Python
	@echo "→ Python tests..."
	@cd $(PYTHON_DIR) && $(PYTHON_ACT) && pytest tests/ -v
	@echo "✓ Python OK"

test-rust: ## Тесты на Rust
	@echo "→ Rust tests..."
	@cd core && cargo test
	@echo "✓ Rust OK"

demo: build-go ## Демо на mock-данных (без Rust-сервера)
	@echo ""
	@echo "════════════════════════════════════════════════════════════"
	@echo "  CodeGraph Demo (mock)"
	@echo "════════════════════════════════════════════════════════════"
	@echo ""
	@echo "── 1. Impact analysis ──"
	@$(GO_BIN) --mock impact --symbol auth.Validate
	@echo ""
	@echo "── 2. Архитектурные правила ──"
	@$(GO_BIN) --mock check --rules rules/go-layers.yaml || true
	@echo ""
	@echo "── 3. Метрики ──"
	@$(GO_BIN) --mock metrics
	@echo ""
	@echo "✓ Демо завершено"

demo-real: build-go run-rust ## Демо с реальным Rust-сервером
	@echo ""
	@echo "════════════════════════════════════════════════════════════"
	@echo "  CodeGraph Demo (real Rust core via gRPC)"
	@echo "════════════════════════════════════════════════════════════"
	@echo ""
	@sleep 1
	@echo "── 1. Index ──"
	@$(GO_BIN) index $(EXAMPLE)
	@echo ""
	@echo "── 2. Impact analysis (real data) ──"
	@$(GO_BIN) impact --symbol auth.Validate
	@echo ""
	@echo "── 3. Метрики (real data) ──"
	@$(GO_BIN) metrics
	@echo ""
	@echo "✓ Демо завершено (Rust-сервер работает в фоне, останови через make stop-rust)"

demo-llm: build-go ## Демо LLM-агента (Analyst, mock)
	@echo ""
	@cd $(PYTHON_DIR) && $(PYTHON_ACT) && \
		codegraph ask --mock --agent analyst "какие метрики у пакета auth?"

demo-llm-verbose: build-go ## LLM-агент с подробным выводом (видно tool calls)
	@cd $(PYTHON_DIR) && $(PYTHON_ACT) && \
		codegraph ask --mock --verbose --agent analyst "что сломается при изменении auth.Validate?"

demo-advisor: build-go ## Демо Advisor (A2A с Analyst)
	@cd $(PYTHON_DIR) && $(PYTHON_ACT) && \
		codegraph ask --mock --agent advisor "как развязать пакеты auth и db?"

install-python: ## Установить Python-клиент (venv + зависимости)
	@echo "→ Установка Python-клиента..."
	@cd $(PYTHON_DIR) && python3 -m venv .venv && \
		$(PYTHON_ACT) && pip install -e ".[dev]"
	@echo "✓ Python-клиент установлен"
	@echo "  Активируй: cd $(PYTHON_DIR) && source .venv/bin/activate"

install-rust: ## Установить Rust (если ещё нет)
	@echo "→ Установка Rust..."
	@curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	@echo "✓ Rust установлен (перезапусти терминал)"

lint-go: ## Линтер Go (нужен golangci-lint)
	@golangci-lint run ./...

lint-python: ## Линтер Python (ruff)
	@cd $(PYTHON_DIR) && $(PYTHON_ACT) && ruff check codegraph_client/ tests/

lint-rust: ## Линтер Rust (clippy)
	@cd core && cargo clippy -- -D warnings

lint: lint-go lint-python lint-rust ## Все линтеры

fmt: ## Форматирование кода (Go + Rust + Python)
	@gofmt -w ./cmd ./internal
	@cd core && cargo fmt
	@cd $(PYTHON_DIR) && $(PYTHON_ACT) && ruff format codegraph_client/ tests/ 2>/dev/null || true
	@echo "✓ Отформатировано"

clean: ## Удалить артефакты сборки
	@echo "→ Удаление артефактов..."
	@rm -rf bin/
	@rm -rf $(PYTHON_DIR)/.venv/
	@rm -rf $(PYTHON_DIR)/.pytest_cache/
	@rm -rf $(PYTHON_DIR)/.ruff_cache/
	@find $(PYTHON_DIR) -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	@find $(PYTHON_DIR) -type d -name '*.egg-info' -exec rm -rf {} + 2>/dev/null || true
	@rm -rf core/target/
	@echo "✓ Чисто"

clean-rust: ## Удалить только Rust-артефакты
	@rm -rf core/target/
	@echo "✓ Rust clean"