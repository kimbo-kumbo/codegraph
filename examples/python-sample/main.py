"""Точка входа примера. Демонстрирует кросс-файловые вызовы."""

from auth import auth


def main():
    username = "alice"
    token = auth.login(username)
    if token:
        print(f"token: {token}")
        auth.logout(username)


if __name__ == "__main__":
    main()