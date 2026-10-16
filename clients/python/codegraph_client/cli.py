"""CLI для CodeGraph Python-клиента.

Команды:
    codegraph ask --agent analyst "какие метрики у auth?"
    codegraph ask --agent advisor "как развязать auth и db?"
    codegraph tools
    codegraph agents
"""

from __future__ import annotations

import asyncio
import os
import sys
import time
from typing import Optional

import typer
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from . import __version__
from .llm.base import Message
from .runtime import Runtime


app = typer.Typer(
    name="codegraph",
    help="CodeGraph Python client — LLM agents for code analysis",
    no_args_is_help=True,
    add_completion=False,
)

console = Console()
err_console = Console(stderr=True)


@app.command()
def ask(
    query: str = typer.Argument(..., help="Вопрос агенту"),
    agent: str = typer.Option(
        "analyst",
        "--agent", "-a",
        help="Агент: analyst | advisor",
    ),
    llm: Optional[str] = typer.Option(
        None,
        "--llm",
        help="LLM: ollama (default) | deepseek | openai | mock",
    ),
    model: Optional[str] = typer.Option(
        None,
        "--model",
        help="Модель. Для ollama: qwen2.5-coder:7b | llama3.1:8b | qwen2.5-coder:14b",
    ),
    verbose: bool = typer.Option(
        False,
        "--verbose", "-v",
        help="Показать tool calls и шаги агента",
    ),
    mock: bool = typer.Option(
        False,
        "--mock",
        help="Использовать mock MCP-сервер (без Rust)",
    ),
) -> None:
    """Задать вопрос агенту.

    Примеры:
        codegraph ask "какие метрики у пакета auth?"
        codegraph ask --agent advisor "как развязать auth и db?"
        codegraph ask --llm ollama --model llama3.1:8b "что сломается при изменении auth.Validate?"
        codegraph ask --llm mock --agent analyst "тест"
    """
    if model:
        os.environ["OLLAMA_MODEL"] = model

    if mock:
        os.environ["CODEGRAPH_MOCK"] = "1"

    asyncio.run(_run_ask(query, agent, llm, verbose))


async def _run_ask(
    query: str,
    agent_name: str,
    llm_name: Optional[str],
    verbose: bool,
) -> None:
    """Реальная реализация ask (async)."""
    try:
        async with Runtime(llm_name=llm_name) as rt:
            if verbose:
                _print_verbose_header(rt, agent_name, query)
                await _run_verbose(rt, agent_name, query)
            else:
                answer = await rt.run_agent(agent_name, query)
                _print_answer(agent_name, answer)
    except KeyboardInterrupt:
        err_console.print("\n[yellow]Прервано пользователем[/yellow]")
        sys.exit(130)
    except Exception as e:
        err_console.print(f"[red]Ошибка:[/red] {type(e).__name__}: {e}")
        if verbose:
            import traceback
            err_console.print(traceback.format_exc())
        sys.exit(1)


def _print_verbose_header(rt: Runtime, agent_name: str, query: str) -> None:
    """Заголовок verbose-режима."""
    console.print()
    console.print(Panel.fit(
        f"[bold]LLM:[/bold] {rt.llm.name}\n"
        f"[bold]Agent:[/bold] {agent_name}\n"
        f"[bold]Query:[/bold] {query}",
        title="[bold cyan]CodeGraph Ask (verbose)[/bold cyan]",
        border_style="cyan",
    ))
    console.print()


async def _run_verbose(rt: Runtime, agent_name: str, query: str) -> None:
    """Verbose-режим: логируем tool calls."""
    agent = rt.registry.get(agent_name)
    tools = await rt.mcp.tools_for_llm()

    messages: list[Message] = [
        Message(role="system", content=agent.system_prompt),
        Message(role="user", content=query),
    ]

    console.print("[dim]→ Отправляю запрос в LLM...[/dim]")
    start = time.monotonic()

    for iteration in range(agent.max_iterations):
        response = await agent.llm.chat(messages, tools=tools, temperature=agent.temperature)

        if not response.tool_calls:
            elapsed = time.monotonic() - start
            console.print(f"[dim]← LLM ответила за {elapsed:.1f}s[/dim]")
            console.print()
            _print_answer(agent_name, response.content or "")
            return

        console.print(f"[bold]→ LLM вызвала {len(response.tool_calls)} tool(s):[/bold]")
        messages.append(Message(
            role="assistant",
            content=response.content,
            tool_calls=response.tool_calls,
        ))

        for tc in response.tool_calls:
            console.print(
                f"  [cyan]{tc.name}[/cyan]("
                f"{', '.join(f'{k}={v!r}' for k, v in tc.arguments.items())}"
                f")"
            )
            result = await rt.mcp.call_tool(tc.name, tc.arguments)
            preview = str(result)[:200].replace("\n", " ")
            console.print(f"    [dim]← {preview}{'...' if len(str(result)) > 200 else ''}[/dim]")
            messages.append(Message(
                role="tool",
                tool_call_id=tc.id,
                content=agent._serialize(result),
            ))

    console.print(f"[red]Достигнут лимит итераций ({agent.max_iterations})[/red]")


def _print_answer(agent_name: str, answer: str) -> None:
    """Красивый вывод ответа."""
    console.print(Panel(
        Text(answer, style="white"),
        title=f"[bold green]{agent_name}[/bold green]",
        border_style="green",
    ))


@app.command()
def tools(
    mock: bool = typer.Option(False, "--mock"),
) -> None:
    """Список MCP-tools (для отладки)."""
    if mock:
        os.environ["CODEGRAPH_MOCK"] = "1"
    asyncio.run(_run_tools())


async def _run_tools() -> None:
    async with Runtime(llm_name="mock") as rt:
        tools_list = await rt.list_tools()
        table = Table(title=f"MCP Tools ({len(tools_list)})")
        table.add_column("Name", style="cyan", no_wrap=True)
        table.add_column("Description", style="white")
        for t in tools_list:
            fn = t["function"]
            desc = fn["description"].split("\n")[0][:80]
            table.add_row(fn["name"], desc)
        console.print(table)

@app.command()
def agents(
    mock: bool = typer.Option(False, "--mock"),
) -> None:
    """Список агентов с Agent Cards (A2A)."""
    if mock:
        os.environ["CODEGRAPH_MOCK"] = "1"
    asyncio.run(_run_agents())


async def _run_agents() -> None:
    async with Runtime(llm_name="mock") as rt:
        cards = rt.list_agents()
        table = Table(title=f"Agents ({len(cards)})")
        table.add_column("Name", style="cyan", no_wrap=True)
        table.add_column("Description")
        table.add_column("Skills", style="dim")
        for c in cards:
            table.add_row(c.name, c.description, ", ".join(c.skills))
        console.print(table)


@app.command()
def version() -> None:
    """Версия клиента."""
    console.print(f"codegraph-client {__version__}")


def main() -> None:
    """Точка входа для скрипта `codegraph`."""
    app()


if __name__ == "__main__":
    main()