# Multi-Stack Full Course

Intro paragraph with <kbd>Cmd</kbd>+<kbd>S</kbd>, 2<sup>10</sup>, H<sub>2</sub>O, and line<br>break.

## Frontend Dev Setup {#ex-1-frontend}

Start the TypeScript web app with explicit fence attributes and verify keyboard shortcuts.

```bash cwd=web-app mode=daemon port=5173 label="Start Web Dev" service_key=web-ui secondary=false
npm run dev
```

```bash cwd=web-app secondary=true
echo "checking workspace"
```

```ts path=web-app/src/index.ts
export const ready = true;
```

```bash cwd=web-app
bun test
```

<details>
<summary>展开提示</summary>

按 <kbd>Cmd</kbd>+<kbd>K</kbd> 打开命令面板，并在 `web-app/src/index.ts` 中检查导出。

</details>

### Verification

- [ ] Dev server responds on `http://127.0.0.1:5173`
- [x] `bun test` passes locally

## Python FastAPI Service {#dup-anchor}

Launch uvicorn and run pytest through `uv run`.

```bash cwd=py-api
uvicorn app.main:app --reload --port 8000
```

```bash cwd=py-api
uv run --extra dev pytest -q
```

```python path=../etc/passwd
print("unsafe path should be stripped to None")
```

Reflection:

- Why should `--reload` stay disabled in production containers?

## Java Spring Boot Service {#dup-anchor}

Boot the Spring service and execute Gradle tests.

```bash cwd=java-api
mvn spring-boot:run
```

```bash cwd=java-api
./gradlew test
```

Acceptance: Service answers on `http://localhost:8081/actuator/health`.
