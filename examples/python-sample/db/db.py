"""Доступ к базе данных."""


def find_user(username: str) -> str:
    """Возвращает запись пользователя по логину."""
    if not username:
        return ""
    return username + "-record"


def query(table: str) -> list:
    """Публичная функция, но никем не вызывается — dead code."""
    return [table]