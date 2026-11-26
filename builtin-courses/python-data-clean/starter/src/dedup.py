from __future__ import annotations

from models import Order


def deduplicate_orders(orders: list[Order]) -> list[Order]:
    """Deduplicate orders by order_id keeping the latest created_at."""
    # TODO: 按 order_id 保留最新记录并按 (created_at, order_id) 升序排序
    return list(orders)
