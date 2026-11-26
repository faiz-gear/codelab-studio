# TypeScript 数据流水线实战

围绕一条多文件 TypeScript 数据处理流水线，逐步完成用户筛选、原始事件清洗、去重排序、类目汇总、游标分页、`Promise.allSettled` 容错收集与有界并发调度。

所有单测文件透明存放在 Workspace 的 `tests/` 目录内，既可点击工作台「运行验收」自动判题（5 秒超时熔断），也能在终端或 VS Code 中直接运行 `bun test` 或 `node --test`。

## 工作目录与使用方式

绑定空目录作为 Workspace 后，工作台会自动复制 Starter 文件：

```text
<workspace>/
├── package.json
├── tsconfig.json
├── src/
│   ├── types.ts        # 共享类型定义
│   ├── filter.ts       # 练习 1
│   ├── normalize.ts    # 练习 2
│   ├── dedup.ts        # 练习 3
│   ├── aggregate.ts    # 练习 4
│   ├── paginate.ts     # 练习 5
│   ├── settled.ts      # 练习 6
│   ├── concurrency.ts  # 练习 7
│   └── pipeline.ts     # 练习 8
└── tests/
    └── ex1-filter.test.ts ... ex8-pipeline.test.ts
```

先在 Workspace 根目录跑一遍全部测试，确认初始状态：

```bash mode=oneshot
bun test
```

## 练习 1：成年用户筛选与不可变映射

补全 `src/filter.ts` 中的 `getAdultNames`，筛出所有 `age >= 18` 的用户并仅返回 `name` 数组，同时绝不修改原数组。

先运行本节单测观察失败断言差异：

```bash mode=oneshot
bun test tests/ex1-filter.test.ts
```

打开 `src/filter.ts`，按以下规则实现 `getAdultNames(users: readonly UserRecord[]): string[]`：

- 保留 `user.age >= 18` 的记录（正好 18 岁计为成年）。
- 仅映射出 `user.name`，保持原顺序。
- 始终返回新数组，不修改传入的 `users` 数组。

**验收：**

- 运行 `tests/ex1-filter.test.ts` 三条用例全部通过。

**思考：**

- `filter` + `map` 链式写法与单次 `for` 循环在可读性与中间数组分配上如何取舍？

## 练习 2：原始事件清洗与字段归一化

补全 `src/normalize.ts` 中的 `normalizeEvents`，清洗脏数据、规范化字符串并将负延迟钳制为 `0`。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex2-normalize.test.ts
```

打开 `src/normalize.ts`，实现 `normalizeEvents(events: readonly RawEvent[]): NormalizedEvent[]`：

- 对 `id` 和 `category` 执行 `trim()`；若任一字段为空则丢弃该条记录。
- `category` 转为小写。
- `tags` 逐项 `trim()` 并转小写，过滤空标签并去重（保持首次出现顺序）。
- `latencyMs < 0` 时钳制为 `0`。

**验收：**

- 运行 `tests/ex2-normalize.test.ts` 三条用例全部通过。

**思考：**

- 为什么归一化逻辑应当放在流水线入口，而不是分散在后续聚合与分页逻辑里？

## 练习 3：按 ID 去重并保留最新事件

补全 `src/dedup.ts` 中的 `dedupKeepLatest`，按 `id` 去重仅保留最新事件并输出确定顺序。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex3-dedup.test.ts
```

打开 `src/dedup.ts`，实现 `dedupKeepLatest(events: readonly NormalizedEvent[]): NormalizedEvent[]`：

- 相同 `id` 的事件仅保留 `timestamp` 更大的记录。
- 若 `timestamp` 相等，保留输入中后出现的那一条（`>=`）。
- 最终数组按 `timestamp` 升序排列；`timestamp` 相同按 `id` 字典序升序排列。

**验收：**

- 运行 `tests/ex3-dedup.test.ts` 三条用例全部通过。

**思考：**

- 当两条事件拥有相同 `id` 和相同时间戳时，为什么要显式规定平手胜出规则？

## 练习 4：按类目分组汇总金额与峰值延迟

补全 `src/aggregate.ts` 中的 `summarizeByCategory`，按类目统计记录数、总金额与最大延迟。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex4-aggregate.test.ts
```

打开 `src/aggregate.ts`，实现 `summarizeByCategory(events: readonly NormalizedEvent[]): Record<string, CategorySummary>`：

- 每个类目记录 `count`、`totalAmount` 和 `maxLatencyMs`。
- 累加金额后四舍五入保留两位小数（`Math.round(sum * 100) / 100`），避免二进制浮点误差。
- 输入为空数组时返回空对象 `{}`。

**验收：**

- 运行 `tests/ex4-aggregate.test.ts` 三条用例全部通过。

**思考：**

- JavaScript 中 `0.1 + 0.2 !== 0.3` 为什么会发生？在高频计费场景下用整数分（cents）有什么优势？

## 练习 5：确定性游标分页器

补全 `src/paginate.ts` 中的 `paginateCursor`，基于唯一 `id` 实现稳定游标分页。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex5-paginate.test.ts
```

打开 `src/paginate.ts`，实现 `paginateCursor<T extends { id: string }>(items: readonly T[], cursor: string | null, limit: number): Page<T>`：

- 若 `limit < 1` 或非整数，抛出 `RangeError`。
- `cursor` 为 `null` 时从第 0 项开始切片。
- `cursor` 非空时，从匹配 `item.id === cursor` 的下一项开始切片；若游标不存在，返回空页（`items: []`, `nextCursor: null`, `hasMore: false`）。
- 当还有剩余元素时 `hasMore` 为 `true` 且 `nextCursor` 为当前页最后一项的 `id`，否则 `nextCursor` 为 `null`。

**验收：**

- 运行 `tests/ex5-paginate.test.ts` 三条用例全部通过。

**思考：**

- 与 `OFFSET / LIMIT` 相比，游标分页在实时插入数据时如何避免漏读和重复读？

## 练习 6：Promise.allSettled 容错批量收集

补全 `src/settled.ts` 中的 `collectSettled`，在部分异步任务失败时仍然完整收集成功结果与错误信息。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex6-settled.test.ts
```

打开 `src/settled.ts`，实现 `collectSettled<T, R>(items: readonly T[], worker: (item: T) => Promise<R>): Promise<BatchResult<R>>`：

- 使用 `Promise.allSettled` 并行执行所有任务，不因单个拒绝而短路。
- `fulfilled` 数组按输入顺序收集所有成功值。
- `rejected` 数组收集所有失败原因的字符串描述（`Error` 取 `error.message`，否则 `String(reason)`）。

**验收：**

- 运行 `tests/ex6-settled.test.ts` 三条用例全部通过。

**思考：**

- 哪些业务场景更适合 `Promise.all`（快速失败），哪些更适合 `Promise.allSettled`（部分容错）？

## 练习 7：有界并发任务调度器

补全 `src/concurrency.ts` 中的 `runWithConcurrency`，限制同时运行中的异步任务数不超过 `limit` 且严格保持结果顺序。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex7-concurrency.test.ts
```

打开 `src/concurrency.ts`，实现 `runWithConcurrency<T, R>(items: readonly T[], limit: number, worker: (item: T) => Promise<R>): Promise<R[]>`：

- 若 `limit < 1` 或非整数，抛出 `RangeError`。
- 同一时刻运行中的 `worker` 调用数不得超过 `limit`。
- 返回结果数组与输入 `items` 下标一一对应，即使靠后的任务先完成。

**验收：**

- 运行 `tests/ex7-concurrency.test.ts` 三条用例全部通过。

**思考：**

- 固定数量 worker 协程拉取共享游标与 `Promise.race` 池两种写法在异常处理上有哪些差异？

## 练习 8：组装端到端异步数据流水线

补全 `src/pipeline.ts` 中的 `runDataPipeline`，串联清洗、去重、有界并发补全与类目汇总，产出最终流水线摘要。

先运行本节单测：

```bash mode=oneshot
bun test tests/ex8-pipeline.test.ts
```

打开 `src/pipeline.ts`，组合前面章节的模块实现 `runDataPipeline`：

1. 调用 `normalizeEvents(raw)` 清洗原始事件。
2. 调用 `dedupKeepLatest` 去重并按时间排序。
3. 调用 `runWithConcurrency(deduped, limit, enrich)` 并发补全事件。
4. 调用 `summarizeByCategory(enriched)` 汇总各类目统计。
5. 计算 `topCategory`：总金额最高的类目（金额相同按类目字典序靠前者优先；无事件时为 `null`）。

**验收：**

- 运行 `tests/ex8-pipeline.test.ts` 三条用例全部通过。

**思考：**

- 在真实生产数据流水线中，如果某一阶段背压过大，如何用异步迭代器（`AsyncIterable`）改造为流式处理？
