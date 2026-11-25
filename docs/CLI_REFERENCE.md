# 🛠️ Zvelte CLI Reference Manual

> Command-line interface documentation for `zvelte` (and its `zvelte-rs` alias).

---

## 💻 Synopsis

```bash
zvelte <command> [path]
```

---

## 📑 Command Summary

| Command | Arguments | Description |
|---|---|---|
| `zvelte new` | `<project-name>` | Creates a new Python-first Zvelte project with strictly `app.toml` and `app.zvelte`. |
| `zvelte check` | `[path]` | Parses `app.toml` and `.zvelte` and runs semantic validation without generating outputs. |
| `zvelte build` | `[path]` | Compiles production assets into `[build] output` (default: `dist/`). |
| `zvelte dev` | `[path]` | Starts the Rust HTTP dev server with port fallback, hot reload watching, and CPython RPC supervisor. |
| `zvelte clean` | `[path]` | Removes configured build output, cache (`.zvelte/`), and legacy `.rustby/` while preserving source files. |
| `zvelte --version` | None | Prints `zvelte version 0.0.6`. |
| `zvelte --help` | None | Displays usage help. |

---

## 📌 Detailed Command Specifications

### `zvelte new <project-name>`
Scaffolds a workspace conforming strictly to the **Two-File Contract**:
- Allows only ASCII alphanumeric characters, `-`, and `_` (1–128 chars; rejects `/`, `\`, `.`, `..`).
- Refuses to overwrite an existing directory.
- Emits strictly `app.toml` (`mode = "python"`) and `app.zvelte`.

### `zvelte check [path]`
- Parses `[path]/app.toml` and the configured entry component.
- Validates mode compatibility (`python`, `rust`, `hybrid`), `<state>` types and uniqueness, `bind:*` targets, `{#each}` collections, and identifiers inside event/`{#if}` expressions.

### `zvelte build [path]`
Writes production artifacts to `[build] output` (default `dist/`):
- `index.html`: HTML5 shell with injected `<style>` CSS (no dev reload script).
- `app.js`: Scoped fine-grained DOM runtime and `POST /api/rpc` client proxies.
- `worker.py`: Standalone CPython stdio JSON-RPC worker (emitted when Python server or native Rust scripts exist).
- `libnative.so` / `libnative.dylib` / `native.dll`: Compiled shared library (emitted only when `<script lang="rust">` is used in `hybrid`/`rust` mode).

### `zvelte dev [path]`
- Compiles the project into `.zvelte/dev/`.
- Launches supervised CPython worker (`[python] executable`, fallback `python3` / `python`) if `worker.py` is generated.
- Binds `host:port` (automatically probing `port..port+9` if busy).
- Watches `app.toml` and the entry `.zvelte` file when `hot_reload = true`, recompiling `.zvelte/dev/`, restarting the Python worker, and triggering browser reload via `/__zvelte_version`.

### `zvelte clean [path]`
- Deletes configured `[build] output` (`dist/`), `[build] cache` (`.zvelte/`), and `.rustby/` safely without modifying `app.toml` or `app.zvelte`.
