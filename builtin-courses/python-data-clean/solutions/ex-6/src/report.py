from __future__ import annotations

from analytics import top_categories
from anomaly import detect_anomalies
from cleaner import sum_valid_by_category
from csv_loader import parse_orders_csv
from dedup import deduplicate_orders


def build_order_report(csv_text: str, top_n: int = 3) -> dict[str, object]:
    """Build an end-to-end order analytics report from raw CSV text."""
    raw_orders, invalid_lines = parse_orders_csv(csv_text)
    anomalies = detect_anomalies(raw_orders)
    deduped = deduplicate_orders(raw_orders)
    valid_orders = [
        order
        for order in deduped
        if order.amount > 0 and order.status.strip().lower() != "cancelled"
    ]
    totals = sum_valid_by_category(deduped)
    ranked = top_categories(deduped, top_n)
    return {
        "valid_order_count": len(valid_orders),
        "invalid_lines": invalid_lines,
        "totals_by_category": totals,
        "top_categories": ranked,
        "anomalies": anomalies,
    }
