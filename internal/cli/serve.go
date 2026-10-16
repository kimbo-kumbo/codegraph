package cli

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"

	"github.com/kimbo-kumbo/codegraph/internal/mcp"
	"github.com/spf13/cobra"
)

var flagServeMCP bool

var serveCmd = &cobra.Command{
	Use:   "serve",
	Short: "Запустить сервер (MCP или HTTP)",
	Long: `Запускает codegraph как долгоживущий сервер.

Режимы:
  --mcp   — MCP-сервер (stdio) для Claude Desktop / Cursor
  --http  — HTTP-сервер (для CI) [пока не реализован]

Пример:
  codegraph serve --mcp
  codegraph --mock serve --mcp

ВАЖНО: MCP-сервер пишет в stdout только MCP-сообщения.
Логи идут в stderr — их видно в терминале, но они не мешают протоколу.`,
	RunE: runServe,
}

func init() {
	rootCmd.AddCommand(serveCmd)
	serveCmd.Flags().BoolVar(
		&flagServeMCP,
		"mcp",
		false,
		"запустить MCP-сервер (stdio)",
	)
}

func runServe(cmd *cobra.Command, _ []string) error {
	if !flagServeMCP {
		return fmt.Errorf("укажите режим: --mcp (HTTP будет позже)")
	}

	ctx, cancel := signal.NotifyContext(cmd.Context(),
		os.Interrupt, syscall.SIGTERM)
	defer cancel()

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	srv := mcp.NewServer(client)

	if err := srv.Serve(ctx); err != nil && ctx.Err() == nil {
		return fmt.Errorf("serve: %w", err)
	}

	return nil
}
