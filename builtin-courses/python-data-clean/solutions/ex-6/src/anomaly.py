from __future__ import annotations

from collections import Counter
from decimal import Decimal

from models import AnomalyReport, Order


def detect_anomalies(
    orders: list[Order],
    threshold_multiplier: Decimal = Decimal("3"),
) -> AnomalyReport:
    """Detect non-positive amounts, duplicate order IDs, and high-value outliers."""
    negative_or_zero_ids = sorted(
        {order.order_id for order in orders if order.amount <= 0}
    )
    counts = Counter(order.order_id for order in orders)
    duplicate_ids = sorted(order_id for order_id, count in counts.items() if count >= 2)

    valid_orders = [
        order
        for order in orders
        if order.amount > 0 and order.status.strip().lower() != "cancelled"
    ]
    high_value_ids: list[str] = []
    if valid_orders:
        mean = sum((order.amount for order in valid_orders), Decimal("0")) / Decimal(
            len(valid_orders)
        )
        threshold = mean * threshold_multiplier
        high_value_ids = sorted(
            {order.order_id for order in valid_orders if order.amount > threshold}
        )

    return AnomalyReport(
        negative_or_zero_ids=negative_or_zero_ids,
        duplicate_ids=duplicate_ids,
        high_value_ids=high_value_ids,
    )
