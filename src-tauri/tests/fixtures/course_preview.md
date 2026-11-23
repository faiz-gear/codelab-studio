# Preview fixture

A short handbook for checking the desktop preview.

## Exercise 0: Start service {#exercise-0-start-service}

Start the sample with <kbd>Enter</kbd>.

```bash cwd=web-app mode=daemon label="Start Web Dev" secondary=false
npm run dev
```

```sh cwd=web-app secondary=true
echo "checking workspace"
```

```ts path=web-app/src/index.ts
export const ready = true;
```

### Verification

- [ ] See the service page

## Exercise 1: Check output

```zsh
cd web-app && bun test
```

[Start service](#exercise-0-start-service)

![Course diagram](diagram.png)

| Command | Output |
| --- | --- |
| Test | Passed |

<details>
<summary>More context</summary>

A second paragraph.

</details>

<script>CHECK_SCRIPT_REMOVED</script>
