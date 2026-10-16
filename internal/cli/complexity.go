package cli

import (
	"context"
	"fmt"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var (
	flagComplexityMinNesting uint32
	flagComplexityRecursion  bool
)

var complexityCmd = &cobra.Command{
	Use:   "find-complexity-patterns",
	Short: "Эвристики на O(n²): вложенные циклы, Contains/sort/append",
	Long: `Ищет паттерны, которые часто приводят к квадратичной сложности:
  - вложенные циклы по коллекциям
  - slices.Contains / sort внутри циклов
  - append внутри вложенных циклов
  - рекурсия (опционально)

ВАЖНО: это ЭВРИСТИКА, а не формальный анализ Big-O.

Пример:
  codegraph find-complexity-patterns
  codegraph find-complexity-patterns --min-nesting-level 3 --include-recursion
  codegraph --mock find-complexity-patterns`,
	RunE: runComplexity,
}

func init() {
	rootCmd.AddCommand(complexityCmd)
	complexityCmd.Flags().Uint32Var(
		&flagComplexityMinNesting,
		"min-nesting-level",
		2,
		"минимальная вложенность циклов",
	)
	complexityCmd.Flags().BoolVar(
		&flagComplexityRecursion,
		"include-recursion",
		false,
		"включить эвристики на рекурсию",
	)
}

func runComplexity(cmd *cobra.Command, _ []string) error {
	ctx, cancel := context.WithTimeout(cmd.Context(), 30*time.Second)
	defer cancel()

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	result, err := client.FindComplexityPatterns(ctx, core.ComplexityRequest{
		MinNestingLevel:  flagComplexityMinNesting,
		IncludeRecursion: flagComplexityRecursion,
	})
	if err != nil {
		return fmt.Errorf("find-complexity-patterns: %w", err)
	}

	fmt.Printf("Найдено паттернов: %d\n\n", result.Total)

	if result.Total == 0 {
		fmt.Println("✓ Ничего не найдено")
		return nil
	}

	for i, p := range result.Patterns {
		fmt.Printf("%d) [%s] %s\n", i+1, p.Severity, p.RuleID)
		if p.Symbol != "" {
			fmt.Printf("   Символ: %s\n", p.Symbol)
		}
		if p.Location.File != "" {
			fmt.Printf("   %s:%d:%d\n", p.Location.File, p.Location.Line, p.Location.Column)
		}
		fmt.Printf("   %s\n", p.Message)
		if p.Suggestion != "" {
			fmt.Printf("   → %s\n", p.Suggestion)
		}
		fmt.Println()
	}

	fmt.Println("⚠ Это эвристика, не формальный анализ Big-O.")
	fmt.Println("  Найденные паттерны стоит проверить вручную.")

	return nil
}
