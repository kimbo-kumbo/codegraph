"""Аутентификация. Намеренно нарушает слои: auth -> db напрямую."""

from db import db


def validate(username: str) -> bool:
    """Проверяет формат имени пользователя."""
    if len(username) < 3:
        return False
    return True


def login(username: str) -> str:
    """Аутентифицирует пользователя.

    Вызывает validate и db.find_user — это ребро CALLS (нарушение слоёв).
    """
    if not validate(username):
        return ""
    user = db.find_user(username)
    return "token-" + user


def logout(username: str) -> bool:
    """Завершает сессию. Вызывает validate."""
    return validate(username)


def hash_password(password: str) -> str:
    """Вспомогательная функция. Не вызывается — dead code."""
    return "hashed:" + password