# task-api 接口合同

任务含正整数 `id`、`title`、`status`。标题去掉首尾空白后必须有 1–100 个 Unicode 码点。
新任务是 `pending`，完成后是 `done`，重复完成返回同一个已完成任务。没有重新打开操作。

| 请求 | 正常结果 | 异常结果 |
| --- | --- | --- |
| POST /tasks | 201，返回新任务 | 标题无效时 400 |
| GET /tasks | 200，按 ID 升序返回数组，空列表为 `[]` | 内部错误时 500 |
| GET /tasks/{id} | 200，返回任务 | ID 无效时 400，任务不存在时 404 |
| PATCH /tasks/{id} | 200，返回改标题后的任务 | ID 或标题无效时 400，任务不存在时 404 |
| POST /tasks/{id}/complete | 200，返回状态为 `done` 的任务 | ID 无效时 400，任务不存在时 404 |
| DELETE /tasks/{id} | 204，无响应体 | ID 无效时 400，任务不存在时 404 |

请求体是一个 JSON 对象。未知字段和多余的 JSON 值返回 400，超过 16 KiB 返回 413。
任务接口的错误响应为 `{"error":"说明"}`，内部错误不包含 SQL、连接信息或堆栈。

## 一次验收示例

1. `POST /tasks`，发送 `{"title":" 学 Go "}`，得到 201 和 `{"id":1,"title":"学 Go","status":"pending"}`。
2. `GET /tasks/1`，得到 200 和同一个任务。 `GET /tasks` 返回包含该任务的数组。
3. `PATCH /tasks/1`，发送 `{"title":"学 GoFrame"}`，得到 200、新标题和 `pending`。
4. 连续两次 `POST /tasks/1/complete`，均得到 200、同样的 ID 和 `done`。
5. `DELETE /tasks/1`，得到 204。再次查询得到 404，列表返回 `[]`。
