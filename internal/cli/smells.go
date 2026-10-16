package cli

import (
	"context"
	"fmt"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var (
	flagSmellsHotPaths  bool
	flagSmellsThreshold uint32
)

var smellsCmd = &cobra.Command{
	Use:   "find-type-smells",
	Short: "Найти type smells: interface{}, рефлексия, глобалы",
	Long: `Ищет «типовые запахи» в коде:
  - interface{} / any в публичных сигнатурах
  - interface{} в горячих путях (высокий fan-in)
  - рефлексия в критичных местах
  - глобальные переменные

Пример:
  codegraph find-type-smells
  codegraph find-type-smells --only-hot-paths --hot-path-threshold 10
  codegraph --mock find-type-smells`,
	RunE: runSmells,
}

func init() {
	rootCmd.AddCommand(smellsCmd)
	smellsCmd.Flags().BoolVar(
		&flagSmellsHotPaths,
		"only-hot-paths",
		false,
		"показывать только в горячих путях (высокий fan-in)",
	)
	smellsCmd.Flags().Uint32Var(
		&flagSmellsThreshold,
		"hot-path-threshold",
		5,
		"порог fan-in: функция считается горячей, если fan-in >= порога",
	)
}

func runSmells(cmd *cobra.Command, _ []string) error {
	ctx, cancel := context.WithTimeout(cmd.Context(), 30*time.Second)
	defer cancel()

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	result, err := client.FindTypeSmells(ctx, core.TypeSmellsRequest{
		OnlyHotPaths:     flagSmellsHotPaths,
		HotPathThreshold: flagSmellsThreshold,
	})
	if err != nil {
		return fmt.Errorf("find-type-smells: %w", err)
	}

	fmt.Printf("Найдено type smells: %d\n\n", result.Total)

	if result.Total == 0 {
		fmt.Println("✓ Ничего не найдено")
		return nil
	}

	for i, s := range result.Smells {
		fmt.Printf("%d) [%s] %s\n", i+1, s.Severity, s.RuleID)
		if s.Symbol != "" {
			fmt.Printf("   Символ: %s\n", s.Symbol)
		}
		if s.Location.File != "" {
			fmt.Printf("   %s:%d:%d\n", s.Location.File, s.Location.Line, s.Location.Column)
		}
		fmt.Printf("   %s\n", s.Message)
		if s.Suggestion != "" {
			fmt.Printf("   → %s\n", s.Suggestion)
		}
		fmt.Println()
	}

	return nil
}
