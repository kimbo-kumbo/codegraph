// Package cli — команды CLI для codegraph.
package cli

import (
	"context"
	"fmt"
	"os"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

const (
	defaultGRPCAddr = "localhost:50051"
	envGRPCAddr = "CODEGRAPH_GRPC_ADDR"
)

var (
	flagMock     bool
	flagGRPCAddr string
)

var rootCmd = &cobra.Command{
	Use:   "codegraph",
	Short: "Code graph analysis for LLM agents",
	Long: `CodeGraph — локальный MCP-сервер и CLI для структурного анализа кодовой базы.

Даёт LLM-агентам структурное понимание кода: impact analysis, архитектурные
правила, dead code, type smells, complexity hints.

Работает локально и приватно. Поддерживает Go, Python, Rust.`,
	SilenceUsage:  true,
	SilenceErrors: true,
}

func Execute() error {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		return err
	}
	return nil
}

func init() {
	rootCmd.PersistentFlags().BoolVar(
		&flagMock,
		"mock",
		false,
		"использовать mock-данные вместо gRPC (для тестов и демо)",
	)
	rootCmd.PersistentFlags().StringVar(
		&flagGRPCAddr,
		"grpc-addr",
		"",
		fmt.Sprintf("адрес gRPC-сервера (по умолчанию %s или $%s)", defaultGRPCAddr, envGRPCAddr),
	)
}

func newClient(ctx context.Context) (core.Client, error) {
	if flagMock {
		return core.NewMockClient(), nil
	}

	addr := flagGRPCAddr
	if addr == "" {
		addr = os.Getenv(envGRPCAddr)
	}
	if addr == "" {
		addr = defaultGRPCAddr
	}

	client, err := core.NewGRPCClient(addr)
	if err != nil {
		return nil, fmt.Errorf("не удалось подключиться к Rust-ядру на %s: %w (используйте --mock для работы без Rust)", addr, err)
	}
	return client, nil
}
