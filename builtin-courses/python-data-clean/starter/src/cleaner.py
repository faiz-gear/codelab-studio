from __future__ import annotations

from decimal import Decimal, ROUND_HALF_UP

from models import Order


def sum_valid_by_category(orders: list[Order]) -> dict[str, Decimal]:
    """Sum valid order amounts grouped by normalized category."""
    totals: dict[str, Decimal] = {}
    for order in orders:
        # TODO: 过滤 amount <= 0、status == "cancelled" 与空类目，并归一化 category
        category = order.category
        totals[category] = (totals.get(category, Decimal("0")) + order.amount).quantize(
            Decimal("0.01"), rounding=ROUND_HALF_UP
        )
    return totals
