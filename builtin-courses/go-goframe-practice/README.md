# Go 与 GoFrame 开发实践手册：从第一性原理出发

学习路线：**围绕同一个任务管理 API，先用 Go 标准库实现，再迁移到 GoFrame，最后验证迁移前后的行为是否一致。**

这样，每学一个框架组件，你都能回答：它解决了什么问题、依赖什么机制、出错后应该去哪里排查。

这份手册可以直接按顺序练习。你已经学过基础语法和并发，不需要重新从变量、循环开始；遇到知识缺口时，再带着具体问题补充。

## 工作目录与使用方式

本学习工作区位于 `/Users/lyle/Work/sandbox/go-goframe-learning/`。所有练习代码、数据库迁移和相关文档均保存在本工作区内。

后续按练习步骤逐步建立以下结构，不必提前创建空目录：

```text
go-goframe-learning/
├── README.md          # 本学习手册
├── task-api/          # 练习 1 中创建：标准库实现
└── task-api-gf/       # 练习 7 中创建：GoFrame 实现
```

开始练习 1 前，先进入本工作区：

```bash
cd /Users/lyle/Work/sandbox/go-goframe-learning
```

练习 7 创建 GoFrame 项目前，也先回到这个工作区，保证两个项目处于同一级目录。

## 学习方法：这里的“第一性原理”是什么

后端服务最基本的工作，是接收输入、按照规则读取或改变状态，再返回结果。实际困难来自几个事实：

- 输入可能无效。
- 多个请求可能同时修改同一份状态。
- 程序会退出，内存里的状态会消失。
- 一次操作可能只执行了一半。
- 客户端可能取消请求，也可能重复发送请求。

后面的路由、校验、锁、数据库、事务和 `context`，都从这些问题中逐步引出。

每次练习固定执行四步：

1. **先预测**：正常、异常、并发情况下，应该发生什么。
2. **再实现**：只完成本次练习需要的能力。
3. **用证据验收**：请求结果、数据库记录或测试结果。
4. **解释原因**：不用框架名词，也能说明实现为什么正确。

不要按“看完多少文档”决定是否进入下一步。以每一步的验收结果为准。

## 练习 0：写清楚系统的规则

先写一页需求，再写代码。

项目叫 `task-api`，核心数据先只有三个字段：

```go
type Task struct {
    ID     int64  `json:"id"`
    Title  string `json:"title"`
    Status string `json:"status"`
}
```

约定如下：

- 标题先用 `strings.TrimSpace` 去掉首尾空白，再检查长度。
- 标题长度为 1～100 个 Unicode 码点，使用 `utf8.RuneCountInString` 计数。
- 新任务状态为 `pending`。
- 完成任务后，状态为 `done`。
- 重复完成同一个任务，仍返回这个已完成的任务。
- 当前版本不提供重新打开任务的操作。

这里的“Unicode 码点”与用户看到的字符不完全相同，例如组合 emoji 可能包含多个码点。先采用这个明确、容易验证的规则。

接口合同如下。成功时直接返回任务对象，列表直接返回数组；错误统一返回 `{"error":"错误说明"}`。

| 请求 | 成功结果 | 需要处理的情况 |
|---|---|---|
| `POST /tasks` | `201`，返回新任务 | 标题不合法返回 `400` |
| `GET /tasks/{id}` | `200`，返回任务 | 不存在返回 `404` |
| `GET /tasks` | `200`，按 ID 升序返回数组 | 没有任务时返回 `[]` |
| `PATCH /tasks/{id}` | `200`，修改标题并返回任务 | 标题不合法返回 `400`；不存在返回 `404` |
| `POST /tasks/{id}/complete` | `200`，返回完成后的任务 | 不存在返回 `404`；重复调用仍成功 |
| `DELETE /tasks/{id}` | `204`，没有响应体 | 不存在返回 `404` |

路径里的 ID 必须是正整数；格式不合法时返回 `400`。服务内部错误返回 `500`，客户端响应不包含原始 SQL、连接信息或堆栈。

这些状态码是本项目采用的接口约定。关键是前后一致，并能被测试验证。

**验收：**不看代码，能够写出“创建任务→查询→改标题→完成→再次完成→删除”的预期请求和响应。

**思考：**“任务不存在”和“数据库无法访问”，为什么不能都返回同一个错误？

## 练习 1：让一个请求完成往返

先理解 HTTP 服务最小需要什么。

在学习工作区中建立项目：

```bash
mkdir task-api
cd task-api
go mod init example.com/task-api
```

创建 `main.go`：

```go
package main

import (
    "encoding/json"
    "log"
    "net/http"
    "time"
)

func main() {
    mux := http.NewServeMux()
    mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
        w.Header().Set("Content-Type", "application/json; charset=utf-8")
        if err := json.NewEncoder(w).Encode(struct {
            Status string `json:"status"`
        }{Status: "ok"}); err != nil {
            log.Printf("write response: %v", err)
        }
    })

    srv := &http.Server{
        Addr:              "127.0.0.1:8080",
        Handler:           mux,
        ReadHeaderTimeout: 5 * time.Second,
    }
    log.Println("listening on http://127.0.0.1:8080")
    log.Fatal(srv.ListenAndServe())
}
```

这里的 `"GET /healthz"` 路由写法要求 Go 1.22 或更高版本。

启动：

```bash
go run .
```

在另一个终端执行：

```bash
curl -i http://127.0.0.1:8080/healthz
curl -i -X POST http://127.0.0.1:8080/healthz
curl -i http://127.0.0.1:8080/missing
```

**验收：**

- 第一个请求返回 `200` 和 `{"status":"ok"}`。
- 第二个请求返回 `405`。
- 第三个请求返回 `404`。
- 能在输出中找到状态行、响应头和响应体。

**思考：**

- `http.Server` 和 `http.ServeMux` 各负责什么？
- 为什么必须在写响应体之前设置 `Content-Type`？
- 这个接口返回成功，能否证明数据库可用？

最后一个问题的答案是不能。当前接口只证明服务可以接收并回应请求。

## 练习 2：在内存中实现任务管理

这一阶段只引入共享状态。

先使用：

```go
type Store struct {
    mu     sync.RWMutex
    nextID int64
    tasks  map[int64]Task
}
```

按顺序实现：

1. 创建任务。
2. 查询单个任务。
3. 查询全部任务。
4. 修改标题。
5. 完成任务。
6. 删除任务。

每完成一个接口，就立即用 `curl` 验证。例如：

```bash
curl -i -X POST http://127.0.0.1:8080/tasks \
  -H 'Content-Type: application/json' \
  -d '{"title":"学习 Go 的 HTTP 服务"}'
```

记下返回的 ID，再调用查询和完成接口。

实现时注意三个边界：

- **锁保护整个操作。** 分配 ID 和写入 map 必须处于同一个写锁保护范围。
- **不要暴露共享数据的可变引用。** 查询时返回任务值；列表在锁内收集数据，然后解锁并排序。
- **列表顺序必须明确。** Go 的 map 遍历不保证顺序，因此要按 ID 排序。

再处理输入边界：

- 请求体最多允许 16 KiB，超过返回 `413`。
- JSON 格式错误返回 `400`。
- 使用 `DisallowUnknownFields` 拒绝拼错的字段。
- 解码一个对象后，再次解码应得到 `io.EOF`，避免接受两个连续的 JSON 对象。
- 空字符串、全空白字符串、超过长度上限的标题都应被拒绝。

可以查阅 `http.MaxBytesReader` 和 `encoding/json.Decoder`，带着这些需求阅读，而不是通读整个包。

**验收：**

- 完成练习 0 的完整操作流程。
- 中文标题按约定正确计数。
- 列表始终按 ID 升序返回。
- 重启程序后，任务消失。

“重启后消失”是这一阶段预期的结果。它会成为引入数据库的理由。

**思考：**为什么 ID 自增使用原子操作后，仍然不能无锁地读写 map？

## 练习 3：把业务规则从 HTTP 中提取出来

现在让规则能够被独立调用和测试。

如果标题规则、状态转换、map 操作和 JSON 输出全挤在处理函数中，修改其中一部分很容易影响其他部分。可以整理成：

```text
task-api/
├── main.go
└── internal/
    ├── task/
    │   ├── task.go
    │   ├── service.go
    │   ├── memory.go
    │   └── service_test.go
    └── httpapi/
        ├── handler.go
        └── handler_test.go
```

职责划分：

- `httpapi`：解析请求，把错误映射到 HTTP 状态，输出 JSON。
- `task`：标题规则、任务操作、业务错误。
- `memory.go`：内存存储和锁。
- `main.go`：创建对象、注册路由、启动服务。

先使用具体类型和普通方法。等下一步出现数据库实现时，再根据真实调用提取小接口。

业务层可以定义：

```go
var ErrNotFound = errors.New("task not found")
```

HTTP 层用 `errors.Is` 判断它。错误需要附加上下文时，用 `%w` 包装，保留可判断的错误链。

添加这些测试：

- 全空白标题被拒绝。
- 100 个中文码点的标题被接受，101 个被拒绝。
- 查询不存在的 ID 得到 `ErrNotFound`。
- 重复完成任务，状态始终是 `done`。
- 并发创建任务，没有丢失任务或重复 ID。

并发测试使用 `sync.WaitGroup` 等同步工具等待任务结束，不用 `time.Sleep` 猜测执行是否完成。HTTP 测试使用 `httptest` 验证状态码和响应体。

执行：

```bash
go test -race ./...
```

**验收：**

- 业务测试不需要启动端口。
- 正常流程和重要异常都有断言。
- 并发测试通过，竞态检测没有报告问题。

**思考：**竞态检测没有发现问题，是否就证明业务操作具备原子性？举一个反例。

## 练习 4：让状态跨越进程重启

这一阶段学习 SQL 和 `database/sql`。

本项目选择 SQLite，便于把注意力放在查询和持久化上。安装驱动：

```bash
go get modernc.org/sqlite@latest
```

代码中注册驱动：

```go
import _ "modernc.org/sqlite"
```

建立第一份数据库迁移文件，例如 `migrations/001_create_tasks.sql`：

```sql
CREATE TABLE tasks (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'pending'
               CHECK (status IN ('pending', 'done')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

保存为文件，是为了让另一位开发者也能建立相同的数据库。不要只在自己的数据库终端里执行后就丢弃。

按以下顺序替换内存存储：

1. `sql.Open` 创建数据库对象。
2. 用带超时的 `PingContext` 检查可用性。
3. 实现创建和单条查询。
4. 实现列表查询，SQL 明确写 `ORDER BY id ASC`。
5. 实现修改、完成和删除。
6. 重跑原来的 HTTP 验收请求。

SQLite 本地练习可以先设置：

```go
db.SetMaxOpenConns(1)
```

这用于控制实验变量；后续再研究多连接下的写竞争与锁等待。

这一阶段必须掌握：

- `sql.DB` 管理的是连接池。
- SQL 使用参数占位符，不用字符串拼接用户输入。
- `QueryRowContext` 返回后，要通过 `Scan` 获取查询错误。
- 多行查询需要关闭 `rows`，并在循环后检查 `rows.Err()`。
- 更新或删除时，要检查 `RowsAffected`，区分成功和不存在。
- 数据库调用传入上层的 `ctx`。

**验收：**

- 创建任务，停止程序，重新启动，任务仍然存在。
- 包含单引号的标题能正常保存。
- 列表顺序不变。
- 数据库错误被记录，客户端收到符合约定的 `500` 响应。

**思考：**`sql.Open` 成功，为什么还不能说明数据库一定可用？

## 练习 5：让一个业务动作中的多次写入保持一致

这一阶段学习事务。

增加一条规则：

> 任务第一次从 `pending` 变为 `done` 时，必须同时记录一条完成事件。两者要么一起成功，要么一起失败。

事件表可以设计为：

```sql
CREATE TABLE task_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id    INTEGER NOT NULL,
    kind       TEXT NOT NULL CHECK (kind = 'completed'),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (task_id, kind)
);
```

本练习保留删除任务之前的历史事件，因此这里保存原任务 ID，不做级联删除。

先分析这个实现：

```text
更新任务状态为 done
插入完成事件
```

如果第二步失败，系统会留下“任务完成，但没有完成事件”的状态。仅仅检查每条 SQL 的错误，无法撤销已经成功的第一步。

正确实现放在同一个事务中：

1. 开始事务。
2. 条件更新：

   ```sql
   UPDATE tasks
   SET status = 'done', updated_at = CURRENT_TIMESTAMP
   WHERE id = ? AND status = 'pending';
   ```

3. 如果影响一行，插入完成事件。
4. 如果影响零行，查询任务，区分“已经完成”和“不存在”。
5. 任意一步失败，回滚。
6. 全部成功后提交，并处理提交错误。

**事务中的查询和写入都使用 `tx`。** 尤其在连接池只有一个连接时，事务持有连接后又通过 `db` 发起查询，可能一直等待可用连接。

添加一个有价值的回归测试：

- 使用独立的测试数据库。
- 创建一条待完成任务。
- 通过测试数据库中的触发器，使完成事件插入必定失败。
- 调用完成操作。
- 验证任务仍为 `pending`，事件不存在。

再验证：

- 完成一次，只有一条事件。
- 重复完成，没有新增事件。
- 并发完成同一任务，最终仍只有一条完成事件；请求错误也必须被检查。

**思考：**事务解决了哪种一致性问题？`UNIQUE(task_id, kind)` 又提供了哪一层保护？

## 练习 6：让请求停止时，工作也能停止

这一阶段把你学过的并发知识放进请求生命周期。

建立完整的上下文传递：

```text
HTTP 请求的 Context
        ↓
业务方法的 Context
        ↓
数据库操作的 Context
```

业务方法开始采用这样的签名：

```go
func (s *Service) Complete(ctx context.Context, id int64) (Task, error)
```

要求：

- 从 `r.Context()` 获取请求上下文。
- 业务方法继续向下传递。
- 数据库使用 `QueryContext`、`ExecContext`、`BeginTx`。
- 派生超时时，调用返回的 `cancel`，通常使用 `defer cancel()`。
- 不在调用链中途换成 `context.Background()`。

做一个可观察的实验：

1. 临时添加开发接口 `/debug/wait`。
2. 用定时器模拟耗时操作。
3. 用 `select` 同时等待定时器与 `ctx.Done()`。
4. 客户端提前取消：

   ```bash
   curl --max-time 0.2 http://127.0.0.1:8080/debug/wait
   ```

5. 验证服务端日志显示取消，处理函数提前返回。
6. 实验结束后移除这个临时接口。

然后实现优雅退出：接收退出信号，使用有超时的 `Server.Shutdown` 等待正在进行的请求结束，再关闭数据库。

官方的 [取消数据库操作说明](https://go.dev/doc/database/cancel-operations) 可以配合这一步阅读。取消是否能及时终止底层数据库工作，还取决于驱动和数据库支持。

**验收：**

- 客户端取消后，模拟工作能提前停止。
- 正常退出时，服务停止接收新请求，并在限定时间内处理退出。
- 能解释每个 goroutine 由谁创建、通过什么条件退出。

## 练习 7：迁移到 GoFrame，先迁移 HTTP 层

现在你已经有一个可作为参照的实现。

保留标准库版本，在学习工作区中新建 GoFrame 项目：

```bash
cd /Users/lyle/Work/sandbox/go-goframe-learning

go install github.com/gogf/gf/cmd/gf/v2@latest
export PATH="$(go env GOPATH)/bin:$PATH"

gf version
gf init task-api-gf -g example.com/task-api-gf
cd task-api-gf
go run main.go
```

命令依据：[官方 CLI 安装说明](https://github.com/gogf/gf/tree/master/cmd/gf)、[项目创建说明](https://goframe.org/docs/cli/init)。

记录 CLI 版本，保留项目的 `go.mod` 和 `go.sum`。练习期间保持版本稳定，便于复现。

迁移顺序：

1. 运行脚手架自带示例，确认项目能启动。
2. 找到启动入口、路由注册位置和配置文件。
3. 定义创建任务的请求、响应结构体。
4. 使用规范路由接收请求。
5. 调用已经写好的业务逻辑。
6. 逐个迁移其余接口。

创建任务的请求可以这样定义：

```go
type CreateReq struct {
    g.Meta `path:"/tasks" method:"post" tags:"Task" summary:"创建任务"`
    Title  string `json:"title" v:"required#标题不能为空"`
}

type CreateRes struct {
    ID     int64  `json:"id"`
    Title  string `json:"title"`
    Status string `json:"status"`
}
```

`g.Meta` 在这里描述接口，`v` 标签执行输入校验。标题去空白和码点长度等业务规则仍然保留在业务层，避免换一个调用入口就绕过规则。

GoFrame 规范路由的处理方法采用：

```go
func(ctx context.Context, req *Request) (res *Response, err error)
```

这一阶段重点阅读 [规范路由的使用说明](https://goframe.org/docs/web/router-registering-strict-router-usage)，理解参数绑定、自动校验、处理方法和响应中间件之间的关系。

特别注意响应合同：

- 观察脚手架或默认响应中间件生成的 JSON 外壳。
- 按练习 0 的约定处理响应结构和 HTTP 状态码。
- 不能把响应体中的业务 `code` 当成 HTTP 状态码。
- 对原先约定拒绝的输入，例如未知字段、尾随 JSON，也要实际验证迁移后的行为。

需要时编写自己的响应中间件。它的职责是把处理结果和错误转换成已约定的 HTTP 响应。

**验收：**

- 第一套验收请求不修改内容，只切换服务地址，就能继续通过。
- `201`、`204`、`400`、`404` 和 `500` 等状态保持一致。
- `204` 没有响应体。
- 能追踪一个请求从进入服务，到参数绑定、业务执行和最终输出的过程。

**思考：**迁移后哪些代码消失了？其中哪些职责由框架承担，哪些规则仍由你负责？

## 练习 8：迁移到 GoFrame ORM

HTTP 层稳定后，再替换数据库访问方式。

按下面的顺序学习 `gdb`：

1. 配置数据库连接。
2. 注册对应数据库驱动。
3. 插入一条任务。
4. 按 ID 查询。
5. 查询有序列表。
6. 条件更新和删除。
7. 事务。

先理解查询，再使用 DAO 代码生成。生成工具能够减少重复代码，但你仍要能判断生成的查询是否正确。

每写一段 ORM 调用，都回答：

- 它会产生什么 SQL？
- 条件有没有限制到目标行？
- 查询字段和排序是否明确？
- 没查到记录时如何表现？
- 更新影响零行时如何区分原因？
- 操作使用的是普通连接还是事务连接？

“完成任务并写入事件”继续保留原来的事务边界。GoFrame 支持事务闭包：闭包返回 `nil` 时尝试提交，返回错误时回滚；外层也要检查事务方法返回的错误。

参见 [GoFrame ORM 事务说明](https://goframe.org/docs/core/gdb-chaining-transaction)。

**验收：**

- 重启后的持久化验收继续通过。
- 强制事件插入失败时，任务状态仍被回滚。
- 重复完成和并发完成没有重复事件。
- 没有把一次业务事务拆到多个独立事务里。

**思考：**调用变短之后，你是否仍能准确说出实际执行的 SQL 和事务范围？

## 练习 9：从“能运行”走到“别人也能运行”

最后完成配置、日志与交付。

增加三项能力：

- **配置**：监听地址、数据库路径、日志级别可以调整。
- **请求日志**：记录请求标识、方法、路径、状态码和耗时；内部错误包含足够的定位信息。
- **运行说明**：从空目录建立数据库、启动程序、运行测试和验证接口的步骤明确。

结合 [GoFrame 工程目录说明](https://goframe.org/docs/design/project-structure) 阅读自己的项目：

- `api` 中放了哪些对外合同？
- `controller` 负责哪些请求处理？
- 哪些规则需要作为业务逻辑复用？
- 数据访问在哪里发生？
- 哪些文件由工具生成，应该通过重新生成来维护？

根据实际职责组织代码。出现复用和隔离需求时再提取层次，避免出现只负责原样转发参数的多层调用。

最后让自己从一个新的数据库文件开始，完成一次交付验收：

- 按说明建立数据库。
- 启动编译后的程序。
- 执行创建、查询、修改、完成、重复完成、删除。
- 验证无效输入和不存在的任务。
- 重启并验证数据保留。
- 执行测试和竞态检测。
- 验证正常退出。

这一轮通过，才算完成这个项目。

## 后续挑战：每次只增加一个会改变设计的问题

| 挑战 | 要推导的问题 | 完成标准 |
|---|---|---|
| 分页与筛选 | 数据量增长后，一次返回全部任务会怎样？ | 分页边界清楚，排序稳定 |
| 用户与权限 | 身份来自哪里，用户可以操作哪些数据？ | 用户 A 无法读取或修改用户 B 的任务 |
| 并发编辑 | 两个客户端修改同一任务，如何发现覆盖？ | 用版本条件更新识别冲突 |
| 批量操作 | 一次处理很多任务，允许多少并发？ | 并发有上限，支持取消，错误可追踪 |
| 部署 | 配置、数据和程序升级如何配合？ | 新环境可按文档启动，数据不会随进程退出消失 |

## 学习记录模板

每次只写四项：

```text
今天要解决的问题：
我预测会发生什么：
实际证据是什么：
我因此修正了什么理解：
```

当你能从“重复请求、并发修改、部分失败、进程退出”这些具体情况推导实现，并用测试证明结果时，你就具备了独立开发 Go 服务的基础。GoFrame 会成为你组织和减少工程代码的工具。

## 教程示例的验证范围

编写本教程时，`/healthz` 示例已实际运行，验证了 JSON 响应、`405` 和 `404`。GoFrame 的命令与关键机制已核对官方文档；当时本机访问 Go 模块代理超时，GoFrame 部分未完成本机运行验证。
