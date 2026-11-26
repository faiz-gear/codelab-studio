from __future__ import annotations

from models import Order


def deduplicate_orders(orders: list[Order]) -> list[Order]:
    """Deduplicate orders by order_id keeping the latest created_at."""
    latest: dict[str, Order] = {}
    for order in orders:
        existing = latest.get(order.order_id)
        if existing is None or order.created_at >= existing.created_at:
            latest[order.order_id] = order
    return sorted(latest.values(), key=lambda item: (item.created_at, item.order_id))
