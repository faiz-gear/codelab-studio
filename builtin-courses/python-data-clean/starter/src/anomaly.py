from __future__ import annotations

from decimal import Decimal

from models import AnomalyReport, Order


def detect_anomalies(
    orders: list[Order],
    threshold_multiplier: Decimal = Decimal("3"),
) -> AnomalyReport:
    """Detect non-positive amounts, duplicate order IDs, and high-value outliers."""
    # TODO: 统计 negative_or_zero_ids、duplicate_ids 与 high_value_ids
    _ = (orders, threshold_multiplier)
    return AnomalyReport(
        negative_or_zero_ids=[],
        duplicate_ids=[],
        high_value_ids=[],
    )
