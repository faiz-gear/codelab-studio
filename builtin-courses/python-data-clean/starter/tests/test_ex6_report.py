from __future__ import annotations

from decimal import Decimal

from models import AnomalyReport
from report import build_order_report


def test_builds_end_to_end_order_report() -> None:
    csv_text = (
        "order_id,category,amount,status,created_at\n"
        "o1,Books,40.00,paid,2026-01-01T09:00:00Z\n"
        "o1,Books,60.00,paid,2026-01-01T10:00:00Z\n"
        "o2,tools,100.00,paid,2026-01-01T11:00:00Z\n"
        "o3,coffee,15.50,cancelled,2026-01-01T12:00:00Z\n"
    )
    summary = build_order_report(csv_text, top_n=2)
    assert summary["valid_order_count"] == 2
    assert summary["invalid_lines"] == []
    assert summary["totals_by_category"] == {
        "books": Decimal("60.00"),
        "tools": Decimal("100.00"),
    }
    assert summary["top_categories"] == [
        ("tools", Decimal("100.00")),
        ("books", Decimal("60.00")),
    ]
    assert summary["anomalies"] == AnomalyReport([], ["o1"], [])


def test_counts_invalid_csv_lines_and_anomalies() -> None:
    csv_text = (
        "order_id,category,amount,status,created_at\n"
        "o1,books,-10.00,paid,2026-01-01T09:00:00Z\n"
        ",books,bad,paid,2026-01-01T10:00:00Z\n"
        "o2,books,25.00,paid,2026-01-01T11:00:00Z\n"
    )
    summary = build_order_report(csv_text, top_n=2)
    assert summary["valid_order_count"] == 1
    assert summary["invalid_lines"] == [3]
    anomalies = summary["anomalies"]
    assert isinstance(anomalies, AnomalyReport)
    assert anomalies.negative_or_zero_ids == ["o1"]


def test_empty_csv_returns_zeroed_report() -> None:
    csv_text = "order_id,category,amount,status,created_at\n"
    assert build_order_report(csv_text) == {
        "valid_order_count": 0,
        "invalid_lines": [],
        "totals_by_category": {},
        "top_categories": [],
        "anomalies": AnomalyReport([], [], []),
    }
