# Java 21 并发限流器实战

基于 JDK 21 与 JUnit 5，围绕生产级 API 限流组件完成 7 个递进式实战练习：从固定令牌桶、时间补充、固定/滑动窗口，到按键隔离、虚拟线程并发压测与双层组合限流器。

所有 JUnit 5 测试源码透明存放在 Workspace 的 `src/test/java/` 目录内；点击工作台「运行验收」将自动调用 `javac` 编译并由 JUnit 5 Console Launcher 产出结构化测试报告（12 秒超时熔断），也可直接在 IntelliJ IDEA 或终端运行同一套测试。

## 工作目录与使用方式

绑定空目录作为 Workspace 后，工作台自动复制标准 Java 工程结构：

```text
<workspace>/
├── pom.xml
└── src/
    ├── main/java/com/codelab/ratelimiter/
    │   ├── Clock.java
    │   ├── RateLimiter.java
    │   ├── RateLimitResult.java
    │   ├── TokenBucket.java              # 练习 1
    │   ├── RefillingTokenBucket.java     # 练习 2
    │   ├── FixedWindowLimiter.java       # 练习 3
    │   ├── SlidingWindowLogLimiter.java  # 练习 4
    │   ├── TenantRateLimiter.java      # 练习 5
    │   ├── VirtualThreadLoadRunner.java  # 练习 6
    │   └── CompositeRateLimiter.java     # 练习 7
    └── test/java/com/codelab/ratelimiter/
        └── Ex1TokenBucketTest.java ... Ex7CompositeRateLimiterTest.java
```

若使用工作台内置托管 JDK 21，可在终端直接查看版本：

```bash mode=oneshot
java -version
```

## 练习 1：线程安全的固定令牌桶

修复 `TokenBucket` 的容量校验与并发扣减缺陷，确保单线程边界准确且高并发下绝不超发。

打开 `src/main/java/com/codelab/ratelimiter/TokenBucket.java`，按规则完善实现：

- 构造参数 `capacity <= 0` 时抛出 `IllegalArgumentException`。
- 使用 `synchronized`（或 CAS）保护 `tryAcquire()` 与 `availableTokens()`。
- 仅当 `tokens > 0` 时扣减 1 并返回 `true`，耗尽后返回 `false` 且余额保持 `0`。

**验收：**

- `Ex1TokenBucketTest` 三条用例全部通过（包含 64 线程并发压测无超发）。

**思考：**

- `synchronized` 与 `AtomicInteger` CAS 循环在竞争激烈与轻竞争场景下各有什么权衡？

## 练习 2：按时间均匀补充的令牌桶

补全 `RefillingTokenBucket`，基于可注入 `Clock` 按恒定速率补充令牌，并在拒绝时计算精确的 `retryAfterMillis`。

打开 `src/main/java/com/codelab/ratelimiter/RefillingTokenBucket.java`，实现规则如下：

- 构造参数：`capacity > 0`、`refillIntervalMillis > 0`（每经过 `refillIntervalMillis` 毫秒补充 1 枚令牌）、`Clock`。
- 每次调用 `tryAcquireResult()` 先按 `(now - lastRefillMillis) / refillIntervalMillis` 补充令牌，上限不超过 `capacity`，并推进 `lastRefillMillis`。
- 若余额 `> 0`，扣减 1 枚并返回 `RateLimitResult(true, tokens, 0)`；否则返回 `RateLimitResult(false, 0, retryAfterMillis)`，其中 `retryAfterMillis = refillIntervalMillis - (now - lastRefillMillis)`（至少为 1）。

**验收：**

- `Ex2RefillingTokenBucketTest` 三条用例全部通过。

**思考：**

- 为什么要推进 `lastRefillMillis += added * refillIntervalMillis` 而不是直接赋值为 `now`？

## 练习 3：固定时间窗口计数器

补全 `FixedWindowLimiter`，在每个长度为 `windowMillis` 的固定时间窗口内限制最多通过 `maxRequests` 次请求。

打开 `src/main/java/com/codelab/ratelimiter/FixedWindowLimiter.java`，实现规则如下：

- 窗口编号为 `windowIndex = now / windowMillis`。
- 当进入新窗口时重置计数为 `0`。
- 若当前窗口已计数 `< maxRequests`，计数加 1 并返回 `RateLimitResult(true, maxRequests - count, 0)`；否则拒绝并返回 `retryAfterMillis = (windowIndex + 1) * windowMillis - now`。

**验收：**

- `Ex3FixedWindowLimiterTest` 三条用例全部通过。

**思考：**

- 固定时间窗口在窗口切换瞬间为什么会出现最高 2 倍额度的“边界突刺”？

## 练习 4：滑动时间窗口日志限流器

补全 `SlidingWindowLogLimiter`，使用双端队列记录最近 `windowMillis` 内的请求时间戳，彻底消除固定窗口边界突刺。

打开 `src/main/java/com/codelab/ratelimiter/SlidingWindowLogLimiter.java`，实现规则如下：

- 每次调用先驱逐队头所有满足 `ts <= now - windowMillis` 的过期时间戳。
- 若队列长度 `< maxRequests`，记录 `now` 并返回 `RateLimitResult(true, maxRequests - deque.size(), 0)`。
- 否则拒绝并返回 `retryAfterMillis = (deque.peekFirst() + windowMillis) - now`。

**验收：**

- `Ex4SlidingWindowLogLimiterTest` 三条用例全部通过。

**思考：**

- 当单窗口允许 10 万次请求时，滑动窗口日志的内存占用会带来什么挑战？工程上如何用滑动窗口计数器近似替代？

## 练习 5：按租户标识隔离的并发限流器

补全 `TenantRateLimiter`，使用 `ConcurrentHashMap.computeIfAbsent` 为不同租户分配独立限流器，避免全局锁串行化。

打开 `src/main/java/com/codelab/ratelimiter/TenantRateLimiter.java`，实现规则如下：

- `tenantId` 为 `null` 或 `trim().isEmpty()` 时抛出 `IllegalArgumentException`。
- 对 `tenantId` 做 `trim()` 后通过 `ConcurrentHashMap.computeIfAbsent` 懒创建该租户专属的 `RateLimiter`。
- 不同租户之间的配额互不干扰，同一租户并发初始化只创建一个实例。

**验收：**

- `Ex5TenantRateLimiterTest` 三条用例全部通过。

**思考：**

- 在高基数 Key（如海量一次性 IP）场景下，如果 `ConcurrentHashMap` 只增不减会导致什么问题？

## 练习 6：Java 21 虚拟线程并发压测器

补全 `VirtualThreadLoadRunner`，使用 Java 21 `Executors.newVirtualThreadPerTaskExecutor()` 并发发起大量请求并汇总通过与拒绝计数。

打开 `src/main/java/com/codelab/ratelimiter/VirtualThreadLoadRunner.java`，实现规则如下：

- 若 `totalRequests < 0` 或 `limiter == null`，抛出 `IllegalArgumentException`。
- 使用 `Executors.newVirtualThreadPerTaskExecutor()` 提交 `totalRequests` 个任务，通过 `CountDownLatch` 对齐起跑以放大并发竞争。
- 用 `AtomicInteger` 统计 `allowedCount` 与 `rejectedCount`，返回 `LoadSummary(totalRequests, allowedCount, rejectedCount)`。

**验收：**

- `Ex6VirtualThreadLoadRunnerTest` 三条用例全部通过。

**思考：**

- 虚拟线程在遇到 `synchronized` 块与 `ReentrantLock` 时在 JDK 21 调度器上的行为有何差异？

## 练习 7：突发与持续双层组合限流器

补全 `CompositeRateLimiter`，同时施加短窗口突发限制与长窗口持续限制，且当任一层拒绝时绝不消耗另一层配额。

打开 `src/main/java/com/codelab/ratelimiter/CompositeRateLimiter.java`，实现规则如下：

- 内部维护突发滑动窗口与持续滑动窗口两个队列，由同一把锁原子协调。
- 每次调用先驱逐两个队列的过期时间戳，再同时检查两层容量。
- 仅当两层均未满时才向两层同时写入当前时间戳并返回允许（`remaining` 取两层剩余量的较小值）。
- 若任一层已满，两层均不写入时间戳，`retryAfterMillis` 取阻塞层的等待时间最大值。

**验收：**

- `Ex7CompositeRateLimiterTest` 三条用例全部通过。

**思考：**

- 如果组合限流器不具备原子“先全检再扣减”语义，客户端持续被长窗口拒绝时会引发什么副作用？
