# Task API · GoFrame

需要 Go 1.22 或更新版本。在 `task-api-gf` 目录运行：

```sh
go mod download
go test -race ./...
go build -o task-api-gf .
./task-api-gf
```

首次启动会在配置指定的位置建立 SQLite 数据库，并依次应用嵌入的迁移文件。
`manifest/config/config.yaml` 中可调整 `server.address`、`database.path` 和 `logger.level`。
要验证全新安装，请为 `database.path` 选择一个尚不存在的文件。
每次请求的 `X-Request-ID` 可与服务日志对应。

另开一个终端执行以下请求。用创建响应中的 ID 替换示例中的 `1`：

```sh
curl -i http://127.0.0.1:8000/tasks
curl -i -X POST http://127.0.0.1:8000/tasks -H 'Content-Type: application/json' -d '{"title":"学习 GoFrame"}'
curl -i http://127.0.0.1:8000/tasks/1
curl -i -X PATCH http://127.0.0.1:8000/tasks/1 -H 'Content-Type: application/json' -d '{"title":"验证 ORM"}'
curl -i -X POST http://127.0.0.1:8000/tasks/1/complete
curl -i -X POST http://127.0.0.1:8000/tasks/1/complete
curl -i -X POST http://127.0.0.1:8000/tasks -H 'Content-Type: application/json' -d '{"title":" "}'
curl -i http://127.0.0.1:8000/tasks/0
```

得到 201 后，两个完成请求都应返回同一个 `done` 任务，末尾两个无效请求都应返回 400。
按 Ctrl-C 停止服务，重新启动，用原有 ID 查询，应仍是 `done`。
随后用 `curl -i -X DELETE http://127.0.0.1:8000/tasks/1` 检查 204 和空响应体，
再查该 ID 应返回 404，列表应为空。
