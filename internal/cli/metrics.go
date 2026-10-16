package cli

import (
	"context"
	"fmt"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var metricsCmd = &cobra.Command{
	Use:   "metrics [package]",
	Short: "Метрики пакета: coupling, instability, abstractness",
	Long: `Показывает метрики пакета (или всех пакетов, если не указан):
  - Fan-in  (Ca) — афферентная связанность
  - Fan-out (Ce) — эфферентная связанность
  - Instability (I) = Ce / (Ca + Ce)
  - Abstractness (A) = abstract / total
  - Distance (D) = |A + I - 1| — distance from main sequence

Пример:
  codegraph metrics
  codegraph metrics auth
  codegraph --mock metrics`,
	Args: cobra.MaximumNArgs(1),
	RunE: runMetrics,
}

func init() {
	rootCmd.AddCommand(metricsCmd)
}

func runMetrics(cmd *cobra.Command, args []string) error {
	ctx, cancel := context.WithTimeout(cmd.Context(), 30*time.Second)
	defer cancel()

	pkg := ""
	if len(args) > 0 {
		pkg = args[0]
	}

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	result, err := client.GetMetrics(ctx, core.MetricsRequest{Package: pkg})
	if err != nil {
		return fmt.Errorf("metrics: %w", err)
	}

	if len(result.Packages) == 0 {
		fmt.Println("Пакеты не найдены.")
		return nil
	}

	fmt.Printf("%-20s %-8s %6s %6s %8s %8s %8s\n",
		"PACKAGE", "LANG", "Ca", "Ce", "I", "A", "D")
	fmt.Println("--------------------------------------------------------------------")

	for _, m := range result.Packages {
		fmt.Printf("%-20s %-8s %6d %6d %8.3f %8.3f %8.3f\n",
			m.Package, m.Language,
			m.FanIn, m.FanOut,
			m.Instability, m.Abstractness, m.Distance,
		)
	}

	fmt.Println()
	fmt.Println("I (Instability) = Ce / (Ca + Ce): 0 = стабильный, 1 = нестабильный")
	fmt.Println("A (Abstractness): 0 = конкретный, 1 = абстрактный")
	fmt.Println("D (Distance) = |A + I - 1|: чем ближе к 0, тем лучше")

	return nil
}
