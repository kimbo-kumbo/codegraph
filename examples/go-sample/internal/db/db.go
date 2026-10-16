// Package db — доступ к базе данных.
package db

import "errors"

func FindUser(username string) (string, error) {
	if username == "" {
		return "", errors.New("empty username")
	}
	return username + "-record", nil
}

func Query(table string) ([]string, error) {
	return []string{table}, nil
}
