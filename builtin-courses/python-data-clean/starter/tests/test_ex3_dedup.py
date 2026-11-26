from __future__ import annotations

from decimal import Decimal

from dedup import deduplicate_orders
from models import Order


def test_keeps_latest_order_by_created_at() -> None:
    orders = [
        Order("ord-1", "books", Decimal("10.00"), "pending", "2026-01-01T08:00:00Z"),
        Order("ord-1", "books", Decimal("10.00"), "paid", "2026-01-01T09:00:00Z"),
        Order("ord-1", "books", Decimal("10.00"), "pending", "2026-01-01T08:30:00Z"),
    ]
    result = deduplicate_orders(orders)
    assert len(result) == 1
    assert result[0].status == "paid"
    assert result[0].created_at == "2026-01-01T09:00:00Z"


def test_breaks_timestamp_ties_with_later_record() -> None:
    orders = [
        Order("ord-1", "books", Decimal("10.00"), "pending", "2026-01-01T09:00:00Z"),
        Order("ord-1", "books", Decimal("12.00"), "paid", "2026-01-01T09:00:00Z"),
    ]
    result = deduplicate_orders(orders)
    assert result[0].amount == Decimal("12.00")
    assert result[0].status == "paid"


def test_sorts_output_by_created_at_then_order_id() -> None:
    orders = [
        Order("ord-b", "books", Decimal("10.00"), "paid", "2026-01-02T00:00:00Z"),
        Order("ord-a", "books", Decimal("20.00"), "paid", "2026-01-02T00:00:00Z"),
        Order("ord-c", "books", Decimal("30.00"), "paid", "2026-01-01T00:00:00Z"),
    ]
    result = deduplicate_orders(orders)
    assert [order.order_id for order in result] == ["ord-c", "ord-a", "ord-b"]
