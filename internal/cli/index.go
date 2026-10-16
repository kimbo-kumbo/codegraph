package cli

import (
	"context"
	"fmt"
	"path/filepath"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var indexCmd = &cobra.Command{
	Use:   "index <path>",
	Short: "Индексировать проект: парсинг + построение графа",
	Long: `Индексирует проект: парсит исходники (Go, Python, Rust),
строит граф кода и сохраняет кэш.

Пример:
  codegraph index ./my-project
  codegraph --mock index ./my-project`,
	Args: cobra.ExactArgs(1),
	RunE: runIndex,
}

func init() {
	rootCmd.AddCommand(indexCmd)
}

func runIndex(cmd *cobra.Command, args []string) error {
	projectPath := args[0]

	ctx, cancel := context.WithTimeout(cmd.Context(), 5*time.Minute)
	defer cancel()

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	fmt.Printf("Индексация проекта: %s\n", projectPath)

	start := time.Now()
	absPath, err := filepath.Abs(projectPath)
	if err != nil {
		return fmt.Errorf("resolve path %q: %w", projectPath, err)
	}

	result, err := client.Index(ctx, core.IndexRequest{
		ProjectPath: absPath,
		Incremental: false,
	})
	if err != nil {
		return fmt.Errorf("index: %w", err)
	}
	elapsed := time.Since(start)

	fmt.Printf("\n✓ Индексация завершена за %s\n", elapsed.Round(time.Millisecond))
	fmt.Printf("  Файлов обработано: %d\n", result.FilesParsed)
	fmt.Printf("  Узлов создано:     %d\n", result.NodesCreated)
	fmt.Printf("  Рёбер создано:     %d\n", result.EdgesCreated)
	if len(result.Errors) > 0 {
		fmt.Printf("\n⚠ Ошибки парсинга (%d):\n", len(result.Errors))
		for _, e := range result.Errors {
			fmt.Printf("  - %s\n", e)
		}
	}

	return nil
}
