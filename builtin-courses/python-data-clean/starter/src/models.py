from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal


@dataclass(frozen=True)
class Order:
    order_id: str
    category: str
    amount: Decimal
    status: str = "paid"
    created_at: str = "2026-01-01T00:00:00Z"


@dataclass(frozen=True)
class AnomalyReport:
    negative_or_zero_ids: list[str]
    duplicate_ids: list[str]
    high_value_ids: list[str]
