from __future__ import annotations

from decimal import Decimal, ROUND_HALF_UP

from models import Order


def sum_valid_by_category(orders: list[Order]) -> dict[str, Decimal]:
    """Sum valid order amounts grouped by normalized category."""
    totals: dict[str, Decimal] = {}
    for order in orders:
        if order.amount <= 0 or order.status.strip().lower() == "cancelled":
            continue
        category = order.category.strip().lower()
        if not category:
            continue
        totals[category] = totals.get(category, Decimal("0")) + order.amount
    return {
        category: total.quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)
        for category, total in totals.items()
    }
