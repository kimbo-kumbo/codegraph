"""Конфигурация Python-клиента.

Определяет:
    - путь к бинарю codegraph
    - режим работы (mock/grpc)
    - адрес gRPC-сервера
    - выбор LLM-провайдера

По умолчанию используется реальный gRPC (нужен запущенный Rust-сервер
на localhost:50051). Для mock-режима установи CODEGRAPH_MOCK=1
или используй флаг --mock в CLI.
"""

from __future__ import annotations

import os
import shutil
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Config:
    """Конфигурация клиента."""
    
    codegraph_binary: Path
    use_mock: bool = False
    grpc_addr: str = "localhost:50051"
    tool_timeout: float = 60.0
    startup_timeout: float = 10.0

    def server_command(self) -> tuple[str, list[str]]:
        """Команда для запуска MCP-сервера (command, args)."""
        args = ["serve", "--mcp"]
        if self.use_mock:
            args.insert(0, "--mock")
        if not self.use_mock and self.grpc_addr:
            args.extend(["--grpc-addr", self.grpc_addr])
        return str(self.codegraph_binary), args


def _find_codegraph_binary() -> Path:
    """Найти бинарь codegraph.

    Порядок поиска:
        1. Переменная CODEGRAPH_BIN.
        2. ../..../../../bin/codegraph относительно этого файла (монорепо).
        3. В $PATH (shutil.which).
    """
    if env := os.getenv("CODEGRAPH_BIN"):
        p = Path(env).expanduser().resolve()
        if p.exists():
            return p
        raise FileNotFoundError(f"CODEGRAPH_BIN={env} не существует")

    here = Path(__file__).resolve()
    candidates = [
        here.parents[3] / "bin" / "codegraph",
        here.parents[2] / "bin" / "codegraph",
        here.parents[1] / "bin" / "codegraph",
    ]
    for c in candidates:
        if c.exists():
            return c

    if which := shutil.which("codegraph"):
        return Path(which)

    raise FileNotFoundError(
        "Не найден бинарь codegraph. "
        "Собери его (`go build -o bin/codegraph ./cmd/codegraph`) "
        "или задай CODEGRAPH_BIN=/path/to/codegraph"
    )


def load_config() -> Config:
    """Загрузить конфиг из env + дефолтов.

    Env:
        CODEGRAPH_MOCK=1 — использовать mock-режим (по умолчанию False).
        CODEGRAPH_GRPC_ADDR — адрес Rust gRPC-сервера (default localhost:50051).
        CODEGRAPH_BIN — путь к бинарю codegraph.
        CODEGRAPH_TOOL_TIMEOUT — таймаут tool call.
    """
    return Config(
        codegraph_binary=_find_codegraph_binary(),
        use_mock=os.getenv("CODEGRAPH_MOCK", "0") == "1",
        grpc_addr=os.getenv("CODEGRAPH_GRPC_ADDR", "localhost:50051"),
        tool_timeout=float(os.getenv("CODEGRAPH_TOOL_TIMEOUT", "60")),
    )