package cli

import (
	"context"
	"fmt"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var flagDeadCodeIncludeExported bool

var deadCodeCmd = &cobra.Command{
	Use:   "find-dead-code",
	Short: "Найти код, недостижимый от entrypoints",
	Long: `Находит функции, которые не вызываются ни от main, ни от роутов, ни от тестов.

По умолчанию показывает только приватные (не exported) функции.
Публичные могут использоваться снаружи пакета — включи их флагом
--include-exported, если хочешь увидеть все.

Пример:
  codegraph find-dead-code
  codegraph find-dead-code --include-exported
  codegraph --mock find-dead-code`,
	RunE: runDeadCode,
}

func init() {
	rootCmd.AddCommand(deadCodeCmd)
	deadCodeCmd.Flags().BoolVar(
		&flagDeadCodeIncludeExported,
		"include-exported",
		false,
		"включать публичные функции (могут использоваться снаружи)",
	)
}

func runDeadCode(cmd *cobra.Command, _ []string) error {
	ctx, cancel := context.WithTimeout(cmd.Context(), 30*time.Second)
	defer cancel()

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	result, err := client.FindDeadCode(ctx, core.DeadCodeRequest{
		IncludeExported: flagDeadCodeIncludeExported,
	})
	if err != nil {
		return fmt.Errorf("find-dead-code: %w", err)
	}

	fmt.Printf("Найдено dead code: %d\n\n", result.Total)

	if result.Total == 0 {
		fmt.Println("✓ Недостижимого кода не найдено")
		return nil
	}

	for i, s := range result.Dead {
		fmt.Printf("%d) %s\n", i+1, s.QualifiedName)
		if s.Location.File != "" {
			fmt.Printf("   %s:%d:%d\n", s.Location.File, s.Location.Line, s.Location.Column)
		}
		if s.Exported {
			fmt.Println("   (публичная — может использоваться снаружи)")
		}
		fmt.Println()
	}

	return nil
}
