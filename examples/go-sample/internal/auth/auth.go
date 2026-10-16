// Package auth — аутентификация пользователей.
package auth

import (
	"errors"

	"example.com/go-sample/internal/db"
)

func Validate(username string) error {
	if len(username) < 3 {
		return errors.New("username too short")
	}
	return nil
}

func Login(username string) (string, error) {
	if err := Validate(username); err != nil {
		return "", err
	}

	user, err := db.FindUser(username)
	if err != nil {
		return "", err
	}
	return "token-" + user, nil
}

func HashPassword(password string) string {
	return "hashed:" + password
}

func Logout(username string) error {
	return Validate(username)
}
