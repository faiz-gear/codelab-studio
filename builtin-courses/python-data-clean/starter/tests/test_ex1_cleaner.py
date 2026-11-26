from __future__ import annotations

from decimal import Decimal

from cleaner import sum_valid_by_category
from models import Order


def test_sums_valid_orders_by_normalized_category() -> None:
    orders = [
        Order("o1", " Books ", Decimal("49.50")),
        Order("o2", "books", Decimal("79.00")),
        Order("o3", "ELECTRONICS", Decimal("199.90")),
    ]
    assert sum_valid_by_category(orders) == {
        "books": Decimal("128.50"),
        "electronics": Decimal("199.90"),
    }


def test_filters_non_positive_and_cancelled_orders() -> None:
    orders = [
        Order("o1", "books", Decimal("99.00"), status="paid"),
        Order("o2", "books", Decimal("-20.00"), status="paid"),
        Order("o3", "books", Decimal("0.00"), status="paid"),
        Order("o4", "books", Decimal("50.00"), status="cancelled"),
        Order("o5", "   ", Decimal("30.00"), status="paid"),
    ]
    assert sum_valid_by_category(orders) == {"books": Decimal("99.00")}


def test_rounds_decimal_totals_half_up() -> None:
    orders = [
        Order("o1", "coffee", Decimal("1.005")),
        Order("o2", "coffee", Decimal("2.000")),
    ]
    assert sum_valid_by_category(orders) == {"coffee": Decimal("3.01")}
