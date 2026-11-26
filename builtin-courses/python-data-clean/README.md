# Python 数据清洗与订单统计

围绕一组多模块电商订单数据清洗任务，使用 Python 标准库（`dataclass`、`Decimal`、`csv`）与 `pytest --json-report` 完成 6 个递进式实战练习。

所有测试文件透明存放在 Workspace 的 `tests/` 目录内；点击工作台「运行验收」即可触发无头 `pytest --json-report` 判题（8 秒超时熔断），并可直接查看单用例耗时与 `Expected vs Actual` 断言差异。

## 工作目录与使用方式

绑定空目录作为 Workspace 后，工作台自动复制 Starter 工程：

```text
<workspace>/
├── pyproject.toml
├── src/
│   ├── models.py       # Order / AnomalyReport 数据模型
│   ├── cleaner.py      # 练习 1
│   ├── csv_loader.py   # 练习 2
│   ├── dedup.py        # 练习 3
│   ├── analytics.py    # 练习 4
│   ├── anomaly.py      # 练习 5
│   └── report.py       # 练习 6
└── tests/
    ├── conftest.py
    └── test_ex1_cleaner.py ... test_ex6_report.py
```

在 Workspace 根目录可随时运行全套测试：

```bash mode=oneshot
pytest -q
```

## 练习 1：清洗无效订单并按类目汇总金额

补全 `src/cleaner.py` 中的 `sum_valid_by_category`，过滤非法与取消订单，并用 `Decimal` 按类目精确累加金额。

先运行本节测试查看失败差异：

```bash mode=oneshot
pytest -q tests/test_ex1_cleaner.py
```

打开 `src/cleaner.py`，按以下约定实现 `sum_valid_by_category(orders: list[Order]) -> dict[str, Decimal]`：

- 忽略 `amount <= 0` 或 `status.strip().lower() == "cancelled"` 的订单。
- 将 `category` 执行 `strip().lower()` 归一化；空类目跳过。
- 使用 `Decimal` 累加并将最终每个类目的总额按 `ROUND_HALF_UP` 量化到两位小数（`Decimal("0.01")`）。

**验收：**

- `tests/test_ex1_cleaner.py` 全部 3 条用例通过。

**思考：**

- 金融金额计算为什么推荐 `Decimal` 而不是内置二进制 `float`？

## 练习 2：使用 dataclass 解析与校验订单 CSV

补全 `src/csv_loader.py` 中的 `parse_orders_csv`，将 CSV 文本解析为 `Order` 列表，并收集格式错误的 1-based 行号。

先运行本节测试：

```bash mode=oneshot
pytest -q tests/test_ex2_csv_loader.py
```

打开 `src/csv_loader.py`，实现 `parse_orders_csv(csv_text: str) -> tuple[list[Order], list[int]]`：

- 表头为 `order_id,category,amount,status,created_at`（第 1 行），数据行从第 2 行开始计数。
- 对各字段执行 `strip()`；`order_id`、`category`、`status`、`created_at` 均不能为空，且 `amount` 必须能被 `Decimal(...)` 合法解析（且非 NaN / Infinity）。
- 合法行构造成 `Order` 加入 `orders`，非法行将 1-based 行号记录到 `invalid_lines`。

**验收：**

- `tests/test_ex2_csv_loader.py` 全部 3 条用例通过。

**思考：**

- 批处理 ETL 任务中，为什么通常选择“收集错误行号继续处理”而不是遇到第一行脏数据就整体崩溃？

## 练习 3：按订单号去重并保留最新状态

补全 `src/dedup.py` 中的 `deduplicate_orders`，按 `order_id` 去重仅保留最新时间记录并输出稳定排序。

先运行本节测试：

```bash mode=oneshot
pytest -q tests/test_ex3_dedup.py
```

打开 `src/dedup.py`，实现 `deduplicate_orders(orders: list[Order]) -> list[Order]`：

- 相同 `order_id` 保留 `created_at`（ISO-8601 字符串）更大的记录；时间相同时保留输入中较晚出现的记录。
- 返回结果按 `(order.created_at, order.order_id)` 升序排列。

**验收：**

- `tests/test_ex3_dedup.py` 全部 3 条用例通过。

**思考：**

- 如果上游时间戳存在不同时区偏移（如 `+08:00` 与 `Z`），直接按字符串比较会有什么风险？

## 练习 4：计算消费额 Top-N 类目排行

补全 `src/analytics.py` 中的 `top_categories`，基于有效订单总额输出前 `top_n` 个类目。

先运行本节测试：

```bash mode=oneshot
pytest -q tests/test_ex4_analytics.py
```

打开 `src/analytics.py`，实现 `top_categories(orders: list[Order], top_n: int) -> list[tuple[str, Decimal]]`：

- `top_n <= 0` 时直接返回空列表 `[]`。
- 复用 `sum_valid_by_category(orders)` 汇总有效类目金额。
- 按金额降序、金额相同时按类目名升序排列，截取前 `top_n` 项返回。

**验收：**

- `tests/test_ex4_analytics.py` 全部 3 条用例通过。

**思考：**

- 当类目数量上百万而 `top_n` 只有 10 时，`heapq.nsmallest` 比全量 `sorted` 在时间复杂度上有什么优势？

## 练习 5：检测异常金额与重复订单

补全 `src/anomaly.py` 中的 `detect_anomalies`，一次性识别非正金额订单、重复订单号和高额离群订单。

先运行本节测试：

```bash mode=oneshot
pytest -q tests/test_ex5_anomaly.py
```

打开 `src/anomaly.py`，实现 `detect_anomalies(orders: list[Order], threshold_multiplier: Decimal = Decimal("3")) -> AnomalyReport`：

- `negative_or_zero_ids`：所有 `amount <= 0` 订单的去重升序 `order_id` 列表。
- `duplicate_ids`：在输入中出现次数 `>= 2` 的去重升序 `order_id` 列表。
- `high_value_ids`：在有效订单（`amount > 0` 且 `status != "cancelled"`）中，计算平均金额 `mean`，收集所有 `amount > mean * threshold_multiplier` 的去重升序 `order_id` 列表。

**验收：**

- `tests/test_ex5_anomaly.py` 全部 3 条用例通过。

**思考：**

- 均值 × 倍数与中位数绝对偏差（MAD）在面对极端异常值时各自有什么表现？

## 练习 6：组装端到端订单统计报表

补全 `src/report.py` 中的 `build_order_report`，将 CSV 解析、去重、类目汇总、Top-N 排行与异常检测组合成统一报表字典。

先运行本节测试：

```bash mode=oneshot
pytest -q tests/test_ex6_report.py
```

打开 `src/report.py`，实现 `build_order_report(csv_text: str, top_n: int = 3) -> dict[str, object]`，返回包含以下键的字典：

- `"valid_order_count"`：去重后有效订单（`amount > 0` 且非 `cancelled`）条数
- `"invalid_lines"`：CSV 非法行号列表
- `"totals_by_category"`：去重后的类目总额字典
- `"top_categories"`：去重后的 Top-N 类目列表
- `"anomalies"`：针对解析出的原始订单列表运行 `detect_anomalies` 得到的 `AnomalyReport`

**验收：**

- `tests/test_ex6_report.py` 全部 3 条用例通过。

**思考：**

- 为什么异常检测要运行在去重之前的原始订单列表上，而金额汇总必须运行在去重之后？
