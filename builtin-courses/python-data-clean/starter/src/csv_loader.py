from __future__ import annotations

from models import Order


def parse_orders_csv(csv_text: str) -> tuple[list[Order], list[int]]:
    """Parse CSV text into valid Order rows and 1-based invalid line numbers."""
    # TODO: 使用 csv.DictReader 校验并解析每一行订单记录
    _ = csv_text
    return [], []
