// Package processor — пример потенциально медленного кода (O(n²)).
package processor

import "slices"

func ProcessAll(items []string, targets []string) []string {
	var result []string
	for _, item := range items {
		for _, target := range targets {
			if slices.Contains(targets, item) {
				result = append(result, item+":"+target)
			}
		}
	}
	return result
}
