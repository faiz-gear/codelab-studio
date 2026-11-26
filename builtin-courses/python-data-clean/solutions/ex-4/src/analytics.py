from __future__ import annotations

from decimal import Decimal

from cleaner import sum_valid_by_category
from models import Order


def top_categories(orders: list[Order], top_n: int) -> list[tuple[str, Decimal]]:
    """Return the top_n categories sorted by descending total and ascending name."""
    if top_n <= 0:
        return []
    totals = sum_valid_by_category(orders)
    ranked = sorted(totals.items(), key=lambda item: (-item[1], item[0]))
    return ranked[:top_n]
