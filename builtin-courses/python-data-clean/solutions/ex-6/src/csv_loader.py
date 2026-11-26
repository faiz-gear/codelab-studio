from __future__ import annotations

import csv
import io
from decimal import Decimal, InvalidOperation

from models import Order


def parse_orders_csv(csv_text: str) -> tuple[list[Order], list[int]]:
    """Parse CSV text into valid Order rows and 1-based invalid line numbers."""
    reader = csv.DictReader(io.StringIO(csv_text))
    orders: list[Order] = []
    invalid_lines: list[int] = []
    for line_no, row in enumerate(reader, start=2):
        order_id = (row.get("order_id") or "").strip()
        category = (row.get("category") or "").strip()
        raw_amount = (row.get("amount") or "").strip()
        status = (row.get("status") or "").strip()
        created_at = (row.get("created_at") or "").strip()
        if not (order_id and category and raw_amount and status and created_at):
            invalid_lines.append(line_no)
            continue
        try:
            amount = Decimal(raw_amount)
        except InvalidOperation:
            invalid_lines.append(line_no)
            continue
        if not amount.is_finite():
            invalid_lines.append(line_no)
            continue
        orders.append(
            Order(
                order_id=order_id,
                category=category,
                amount=amount,
                status=status,
                created_at=created_at,
            )
        )
    return orders, invalid_lines
