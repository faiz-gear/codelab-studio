from __future__ import annotations

from decimal import Decimal

from anomaly import detect_anomalies
from models import AnomalyReport, Order


def test_identifies_non_positive_and_duplicate_order_ids() -> None:
    orders = [
        Order("o1", "books", Decimal("20.00")),
        Order("o2", "books", Decimal("-5.00")),
        Order("o3", "books", Decimal("0.00")),
        Order("o1", "books", Decimal("20.00")),
    ]
    report = detect_anomalies(orders)
    assert report.negative_or_zero_ids == ["o2", "o3"]
    assert report.duplicate_ids == ["o1"]


def test_flags_high_value_outliers_above_threshold() -> None:
    orders = [
        Order("o1", "books", Decimal("10.00")),
        Order("o2", "books", Decimal("10.00")),
        Order("o3", "books", Decimal("10.00")),
        Order("o4", "books", Decimal("10.00")),
        Order("o5", "books", Decimal("10.00")),
        Order("o6", "books", Decimal("500.00")),
        Order("o7", "books", Decimal("900.00"), status="cancelled"),
    ]
    report = detect_anomalies(orders, threshold_multiplier=Decimal("3"))
    assert report.high_value_ids == ["o6"]


def test_clean_dataset_produces_empty_anomaly_report() -> None:
    orders = [
        Order("o1", "books", Decimal("25.00")),
        Order("o2", "tools", Decimal("30.00")),
    ]
    assert detect_anomalies(orders) == AnomalyReport([], [], [])
