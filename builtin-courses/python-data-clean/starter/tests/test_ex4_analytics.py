from __future__ import annotations

from decimal import Decimal

from analytics import top_categories
from models import Order


def test_returns_top_n_categories_by_total_desc() -> None:
    orders = [
        Order("o1", "books", Decimal("50.00")),
        Order("o2", "tools", Decimal("120.00")),
        Order("o3", "coffee", Decimal("30.00")),
        Order("o4", "books", Decimal("40.00")),
    ]
    assert top_categories(orders, 2) == [
        ("tools", Decimal("120.00")),
        ("books", Decimal("90.00")),
    ]


def test_breaks_equal_totals_alphabetically() -> None:
    orders = [
        Order("o1", "music", Decimal("80.00")),
        Order("o2", "books", Decimal("80.00")),
        Order("o3", "games", Decimal("80.00")),
    ]
    assert top_categories(orders, 2) == [
        ("books", Decimal("80.00")),
        ("games", Decimal("80.00")),
    ]


def test_non_positive_top_n_returns_empty_list() -> None:
    orders = [Order("o1", "books", Decimal("50.00"))]
    assert top_categories(orders, 0) == []
    assert top_categories(orders, -2) == []
