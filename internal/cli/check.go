package cli

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/spf13/cobra"
)

var (
	flagCheckRulesFile string
	flagCheckFormat    string
)

var checkCmd = &cobra.Command{
	Use:   "check --rules <file>",
	Short: "Проверить архитектурные правила из YAML",
	Long: `Проверяет архитектурные правила (слои, запрет циклов, границы пакетов),
описанные в YAML-файле.

Пример:
  codegraph check --rules rules/go-layers.yaml
  codegraph --mock check --rules rules/go-layers.yaml`,
	RunE: runCheck,
}

func init() {
	rootCmd.AddCommand(checkCmd)
	checkCmd.Flags().StringVar(
		&flagCheckRulesFile,
		"rules",
		"",
		"путь к YAML-файлу с правилами",
	)
	checkCmd.Flags().StringVar(
		&flagCheckFormat,
		"format",
		"text",
		"формат вывода: text | sarif",
	)
	_ = checkCmd.MarkFlagRequired("rules")
}

func runCheck(cmd *cobra.Command, _ []string) error {
	ctx, cancel := context.WithTimeout(cmd.Context(), time.Minute)
	defer cancel()

	// TODO: парсинг YAML из internal/rules (сделаем позже).
	// Пока — заглушка: пустой список правил.
	_ = flagCheckRulesFile
	var parsedRules []core.ArchitectureRule

	client, err := newClient(ctx)
	if err != nil {
		return err
	}
	defer client.Close()

	result, err := client.CheckArchitecture(ctx, parsedRules)
	if err != nil {
		return fmt.Errorf("check: %w", err)
	}

	if flagCheckFormat == "sarif" {
		fmt.Fprintln(os.Stdout, `{"version":"2.1.0","runs":[]}`)
		return nil
	}

	fmt.Printf("Проверено правил: %d\n", result.RulesChecked)
	fmt.Printf("Нарушений:        %d\n\n", result.ViolationsCount)

	if result.ViolationsCount == 0 {
		fmt.Println("✓ Нарушений не найдено")
		return nil
	}

	for i, v := range result.Violations {
		fmt.Printf("%d) [%s] %s\n", i+1, v.Severity, v.RuleID)
		if v.Symbol != "" {
			fmt.Printf("   Символ: %s\n", v.Symbol)
		}
		if v.Location.File != "" {
			fmt.Printf("   %s:%d:%d\n", v.Location.File, v.Location.Line, v.Location.Column)
		}
		fmt.Printf("   %s\n", v.Message)
		if v.Suggestion != "" {
			fmt.Printf("   → %s\n", v.Suggestion)
		}
		fmt.Println()
	}

	for _, v := range result.Violations {
		if v.Severity == core.SeverityError {
			return fmt.Errorf("обнаружены нарушения уровня error")
		}
	}
	return nil
}
