package main

import (
	"fmt"

	"example.com/go-sample/internal/auth"
)

func main() {
	user := "alice"
	token, err := auth.Login(user)
	if err != nil {
		fmt.Println("login failed:", err)
		return
	}
	fmt.Println("token:", token)
}
