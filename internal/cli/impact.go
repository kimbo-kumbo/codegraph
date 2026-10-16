package cli

import (
	"context"
	"fmt"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var (
	flagImpactMaxDepth     uint32
	flagImpactIncludeTests bool
)

var impactCmd = &cobra.Command{
	Use:   "impact --symbol <symbol>",
	Short: "Blast radius: что сломается при изменении символа",
	Long: `Анализирует, что затронет изменение символа: все функции,
пакеты и HTTP-роуты, которые от него зависят.

Пример:
  codegraph impact --symbol auth.Validate
  codegraph --mock impact --symbol auth.Validate
  codegraph impact --symbol auth.Validate --max-depth 3 --include-tests`,
	RunE: runImpact,
}

func init() {
	rootCmd.AddCommand(impactCmd)
	impactCmd.Flags().StringVar(
		&flagImpactSymbol,
		"symbol",
		"",
		"символ для анализа (например, auth.Validate)",
	)
	impactCmd.Flags().Uint32Var(
		&flagImpactMaxDepth,
		"max-depth",
		0,
		"максимальная глубина обхода (0 = без ограничения)",
	)
	impactCmd.Flags().BoolVar(
		&flagImpactIncludeTests,
		"include-tests",
		false,
		"включать тесты в результат",
	)
	_ = impactCmd.MarkFlagRequired("symbol")
}

var flagImpactSymbol string

func runImpact(cmd *cobra.Command, _ []string) error {
	ctx, cancel := context.WithTimeout(cmd.Context(), 30*time.Second)
	defer cancel()

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	impact, err := client.ImpactOfChange(ctx, core.ImpactRequest{
		Symbol:       flagImpactSymbol,
		MaxDepth:     flagImpactMaxDepth,
		IncludeTests: flagImpactIncludeTests,
	})
	if err != nil {
		return fmt.Errorf("impact: %w", err)
	}

	fmt.Printf("Символ: %s\n", impact.Symbol.QualifiedName)
	if impact.Symbol.Location.File != "" {
		fmt.Printf("  %s:%d\n", impact.Symbol.Location.File, impact.Symbol.Location.Line)
	}
	fmt.Println()

	if impact.TotalFunctions == 0 {
		fmt.Println("Ничего не зависит от этого символа.")
		return nil
	}

	fmt.Printf("Затронуто функций:  %d\n", impact.TotalFunctions)
	fmt.Printf("Затронуто пакетов:  %d\n", impact.TotalPackages)
	if len(impact.AffectedRoutes) > 0 {
		fmt.Printf("Затронуто роутов:   %d\n", len(impact.AffectedRoutes))
	}
	fmt.Println()

	if len(impact.AffectedPackages) > 0 {
		fmt.Println("Пакеты:")
		for _, p := range impact.AffectedPackages {
			fmt.Printf("  - %s\n", p)
		}
		fmt.Println()
	}

	if len(impact.Affected) > 0 {
		fmt.Println("Функции:")
		for _, s := range impact.Affected {
			loc := ""
			if s.Location.File != "" {
				loc = fmt.Sprintf(" (%s:%d)", s.Location.File, s.Location.Line)
			}
			fmt.Printf("  - %s%s\n", s.QualifiedName, loc)
		}
	}

	if len(impact.AffectedRoutes) > 0 {
		fmt.Println()
		fmt.Println("HTTP-роуты:")
		for _, r := range impact.AffectedRoutes {
			fmt.Printf("  - %s\n", r.QualifiedName)
		}
	}

	return nil
}
