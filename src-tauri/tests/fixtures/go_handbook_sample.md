# Go & GoFrame 工程实战手册

本手册通过 10 个循序渐进的练习，带你从 Go 标准库过渡到 GoFrame 生产级 Web 服务开发。

| 阶段 | 目录 | 核心目标 |
| --- | --- | --- |
| 基础服务 | `task-api` | `net/http`、并发测试、`database/sql` |
| 框架迁移 | `task-api-gf` | `gf init`、路由分组、统一响应 |

参考官方文档：[Go Documentation](https://go.dev/doc/) 与 [GoFrame Docs](https://goframe.org/)。

## 练习 0：写清楚系统的规则（Contract First）

本节先定义任务服务的 HTTP 接口契约和错误码规范，确保后续每一步都可验收。

在开始前先准备本地目录（绝对路径不会越界到工作区外）：

```bash
cd /Users/demo/workspace
```

初始化工程骨架：

```bash
mkdir -p task-api
cd task-api
go mod init task-api
```

**验收：**确认 `go.mod` 已生成且模块名为 `task-api`。

**思考：**为什么工程实践里要先写清楚接口契约，再动手写业务代码？

## 练习 1：用标准库启动 `/healthz` 服务

在 `task-api` 中编写最小 HTTP 服务，监听 `:8080` 并暴露 `/healthz` 探针。

进入目录并创建 `main.go`：

```bash
cd task-api
```

创建 `main.go`：

```go
package main

import (
	"fmt"
	"net/http"
	"time"
)

func main() {
	mux := http.NewServeMux()
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = fmt.Fprint(w, `{"status":"ok"}`)
	})

	srv := &http.Server{
		Addr:              ":8080",
		Handler:           mux,
		ReadHeaderTimeout: 3 * time.Second,
	}
	_ = srv.ListenAndServe()
}
```

启动服务：

```bash
go run .
```

验证健康检查接口：

```bash
curl -i http://127.0.0.1:8080/healthz
```

**验收：**

- `GET /healthz` 返回状态码 200
- 响应体包含 `{"status":"ok"}`

**思考：**为什么生产环境不能直接用 `http.ListenAndServe`，而要显式配置 `ReadHeaderTimeout`？

## 练习 2：路由拆分与请求校验

把路由与处理器拆到独立文件中，并补充 JSON 请求体校验。

创建 `task-api/internal/handler.go`：

```go
package internal

import "net/http"

func HealthHandler(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}
```

运行单元测试：

```bash
cd task-api
go test ./...
```

**验收：**请求缺少必填字段时返回 400。

## 练习 3：内存存储与并发安全测试

使用 `sync.RWMutex` 保护任务列表，并通过 `-race` 验证并发读写零竞态。

进入目录并编写存储实现：

```bash
cd task-api
```

修改 `store.go`：

```go
package main

import "sync"

type Store struct {
	mu    sync.RWMutex
	items []string
}
```

运行竞态检测测试：

```bash
go test -race ./...
```

**验收：**`go test -race ./...` 全部通过且无 `WARNING: DATA RACE`。

## 练习 4：Context 超时控制与嵌套探测

给慢接口加上 `context.WithTimeout`，避免下游阻塞拖垮整个服务。

```bash
cd task-api
go run .
```

按照下面步骤验证超时行为：

1. 发送一个短超时请求观察服务端取消：

   ```bash
   curl --max-time 0.2 http://127.0.0.1:8080/slow
   ```

2. 观察日志是否打印 `context canceled`。

**验收：**客户端超时后服务端能在 200ms 内中止处理。

## 练习 5：接入 SQLite 与 `database/sql`

使用 `database/sql` 连接本地数据库并执行建表迁移。

创建 `task-api/schema.sql`：

```sql
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL
);
```

执行数据库测试：

```bash
cd task-api
go test ./internal/db/...
```

**验收：**重启服务后历史任务数据依然存在。

## 练习 6：结构化日志与请求追踪

为每个请求注入 `X-Request-ID` 并在中间件中记录延迟毫秒数。

```bash
cd task-api
go test -v ./...
```

**验收：**每条请求日志都包含 `request_id` 与 `latency_ms`。

## 练习 7：优雅关闭与目录回退

监听 `SIGINT` / `SIGTERM`，在退出前调用 `srv.Shutdown(ctx)` 排空在途请求。

```bash
cd task-api/internal
cd ..
go test -race ./...
```

**验收：**收到 `SIGINT` 时在途请求能完整返回，不直接断连。

## 练习 8：用 GoFrame 重建任务服务

使用 `gf init` 初始化 GoFrame 工程，对比标准库与框架在工程结构上的差异。

初始化 GoFrame 项目：

```bash
gf init task-api-gf
```

进入目录并启动 GoFrame 服务：

```bash
cd task-api-gf
go run main.go
```

验证 GoFrame 健康探针：

```bash
curl -s http://127.0.0.1:8000/healthz
```

**验收：**GoFrame 服务成功在 `127.0.0.1:8000` 响应 `/healthz`。

### 课后思考

- GoFrame 自动生成规范路由带来了哪些收益？
- 哪些底层并发与超时原则在迁移到框架后依然必须自己守住？

## 练习 9：交付自查与回归验证

对标准库与 GoFrame 两个版本运行完整回归测试，完成最终交付。

```bash
cd task-api
go test -race -count=1 ./...
```

完成一次交付验收：

- 所有单元测试与竞态测试全部通过
- 接口契约与错误码在两个版本中保持一致
- 文档与启动命令均可一键复现
