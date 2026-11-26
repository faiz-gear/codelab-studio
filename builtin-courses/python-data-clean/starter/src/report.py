from __future__ import annotations

from models import AnomalyReport


def build_order_report(csv_text: str, top_n: int = 3) -> dict[str, object]:
    """Build an end-to-end order analytics report from raw CSV text."""
    # TODO: 串联 CSV 解析、去重、类目汇总、Top-N 排行与异常检测
    _ = (csv_text, top_n)
    return {
        "valid_order_count": 0,
        "invalid_lines": [],
        "totals_by_category": {},
        "top_categories": [],
        "anomalies": AnomalyReport([], [], []),
    }
