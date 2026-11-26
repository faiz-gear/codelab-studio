from __future__ import annotations

from decimal import Decimal

from models import Order


def top_categories(orders: list[Order], top_n: int) -> list[tuple[str, Decimal]]:
    """Return the top_n categories sorted by descending total and ascending name."""
    # TODO: 复用 sum_valid_by_category 并按 (-total, category) 排序截取前 top_n 项
    _ = (orders, top_n)
    return []
