from __future__ import annotations

from decimal import Decimal

from csv_loader import parse_orders_csv
from models import Order


def test_parses_valid_csv_rows_into_orders() -> None:
    csv_text = (
        "order_id,category,amount,status,created_at\n"
        " ord-1 , Books , 42.50 , paid , 2026-01-01T10:00:00Z \n"
        "ord-2,tools,19.99,paid,2026-01-02T11:00:00Z\n"
    )
    orders, invalid = parse_orders_csv(csv_text)
    assert invalid == []
    assert orders == [
        Order("ord-1", "Books", Decimal("42.50"), "paid", "2026-01-01T10:00:00Z"),
        Order("ord-2", "tools", Decimal("19.99"), "paid", "2026-01-02T11:00:00Z"),
    ]


def test_records_one_based_line_numbers_for_invalid_rows() -> None:
    csv_text = (
        "order_id,category,amount,status,created_at\n"
        "ord-1,books,10.00,paid,2026-01-01T10:00:00Z\n"
        ",books,15.00,paid,2026-01-01T11:00:00Z\n"
        "ord-3,books,not-a-number,paid,2026-01-01T12:00:00Z\n"
        "ord-4,books,NaN,paid,2026-01-01T13:00:00Z\n"
        "ord-5,tools,25.00,paid,2026-01-01T14:00:00Z\n"
    )
    orders, invalid = parse_orders_csv(csv_text)
    assert [order.order_id for order in orders] == ["ord-1", "ord-5"]
    assert invalid == [3, 4, 5]


def test_handles_header_only_csv() -> None:
    csv_text = "order_id,category,amount,status,created_at\n"
    orders, invalid = parse_orders_csv(csv_text)
    assert orders == []
    assert invalid == []
