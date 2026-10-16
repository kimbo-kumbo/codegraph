package main

import (
	"os"

	"github.com/kimbo-kumbo/codegraph/internal/cli"
)

func main() {
	if err := cli.Execute(); err != nil {
		os.Exit(1)
	}
}
