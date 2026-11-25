# Coding Standards

Apply every rule below when reviewing a diff for CodeLab Studio. `pnpm run check:all` runs the automated checks, including the architecture, contract, and design token guards.

## Architecture & Seams

- **Single domain authority (`codelab-core`)**: Domain validation, path canonicalization, trust normalization, and state transitions belong exclusively in `crates/codelab-core`. Keep `src/` limited to rendering and transient UI state; avoid duplicating domain validation rules in TypeScript.
- **Canonical root containment** covers every file read from a Course Pack or Workspace, including `codelab.json`, `.codelab-checks.json`, Markdown entries, Starter files, Reference Solution snapshots, and learner source files. Use a containment helper before opening each file. Verify that the resolved path stays within the canonical root, even when the input contains symlinks or `..` segments.
- **Bound untrusted bytes before decoding**: Enforce size caps (`MAX_WORKSPACE_TEXT_BYTES`, `MAX_MARKDOWN_BYTES`) via metadata and bounded `.take()` reads before allocating buffers or running UTF-8/JSON parsers.
- **Zero-logic Tauri shell (`src-tauri`)**: Forward IPC commands, event streams, native dialogs, and lifecycle hooks directly to `codelab_core::App` without wrapper middle-man structs or business branching.
- **Test at the two primary seams**: Exercise `codelab-core` through its public `App` interface (`crates/codelab-core/tests/`) using real temp directories, loopback servers, and child processes; exercise React UI through the `CoreClient` interface (`src/__tests__/`) using `FakeCoreClient`. When two failure sources share an `ErrorCode` (for example, Course snapshot vs Workspace target validation), keep the non-tested side valid so each assertion isolates the path under test.

## Crash-Safe Persistence

- **Load persisted JSON as raw bytes**, decode with `serde_json::from_slice`, and validate domain invariants. Quarantine invalid UTF-8, truncated JSON, unsupported `schemaVersion`, and out-of-range values in `.corrupt.*` backups and recover with valid defaults.
- **Replace persisted JSON atomically** on POSIX and Windows. Write a temporary file in the destination directory, call `sync_all` on the file, and replace the destination atomically. Sync the parent directory on Unix. Resolve all `.codelab` reads, writes, and recovery moves relative to the validated metadata directory handle.

## Errors & Localization

- **Code-only `DomainError` across IPC**: Return structured `DomainError { code, details }` from Rust with zero hardcoded display prose.
- **Safe bilingual rendering (`zh-CN` / `en-US`)**: Render every `DomainError` through `formatDomainError`, falling back to `UNEXPECTED_ERROR` whenever an error code is unrecognized or a `{placeholder}` lacks its detail so raw codes and unresolved `{key}` tokens never surface in UI.

## Domain Language & UI Craft

- **Glossary & ADR fidelity**: Name types, commands, events, and UI concepts with `GLOSSARY.md` vocabulary (`Course`, `Course Pack`, `Workspace`, `Exercise`, `Trust State`, `Tutor Session`). Follow `docs/adr/0001`–`0009` where legacy `PRODUCT.md`, `docs/prd/`, or `docs/design/prototype/` differ (notably ADR-0004 ACP-only tutor and ADR-0009 `guided_readonly`).
- **Arc UI v3.2 craft**: Keep neutral surfaces zero-chroma (`oklch(L% 0 0)`), render live ratios/counters/paths in tabular `.mono`, preserve visible double-ring `:focus-visible` states over resting or pressed shadows, and hydrate cached appearance state synchronously to prevent theme or locale startup flash.
