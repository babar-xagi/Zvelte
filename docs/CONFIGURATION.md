# ⚙️ `app.toml` Configuration Reference

> Complete specification for `app.toml`, the single authoritative configuration file for Zvelte projects.

---

## 📄 Example Configuration (`mode = "python"`)

```toml
[project]
name = "my-app"
version = "0.1.0"
entry = "app.zvelte"

[app]
target = "web"
mode = "python"              # Options: "python", "rust", "hybrid"
renderer = "js"              # Currently active: "js" ("wasm" planned)
strict = true

[python]
enabled = true
executable = "python3"       # Command or path for CPython supervisor
version = "3.10+"

[tailwind]
enabled = true
preflight = true
dark_mode = "media"          # Options: "media", "class"
mode = "auto"                # Options: "auto", "builtin", "cli", "off"

[dev]
host = "127.0.0.1"
port = 3000
hot_reload = true            # Watches app.toml and entry component for changes
open_browser = false

[build]
output = "dist"              # Alias: out_dir
cache = ".zvelte"
optimization = "release"
minify = false
source_maps = true
```

---

## 🔍 Section Breakdown

### 1. `[project]`
| Key | Type | Default | Description |
|---|---|---|---|
| `name` | string | `"zvelte-app"` | Application name (also sets HTML `<title>`). |
| `version` | string | `"0.1.0"` | Semantic version string. |
| `entry` | string | `"app.zvelte"` | Entry single-file component path. |

### 2. `[app]`
| Key | Type | Default | Description |
|---|---|---|---|
| `target` | string | `"web"` | Target platform. |
| `mode` | string | `"hybrid"` | Permitted script languages: `"python"` (Python + JS), `"rust"` (Rust + JS), or `"hybrid"` (all). |
| `renderer` | string | `"js"` | Client renderer (`"js"` active; `"wasm"` reserved on roadmap). |
| `strict` | bool | `true` | Strict configuration/semantic enforcement flag. |

- **`mode = "python"`**: Rejects `<script lang="rust">` blocks (`RustBlockNotAllowedInPythonMode`).
- **`mode = "rust"`**: Rejects `<script lang="python">` blocks (`PythonBlockNotAllowedInRustMode`).
- **`mode = "hybrid"`**: Allows both Python and optional native Rust blocks.

### 3. `[python]` & `[toolchain]`
| Section / Key | Type | Default | Description |
|---|---|---|---|
| `[python] enabled` | bool | `true` | Enables Python configuration. |
| `[python] executable` | string | `"python3"` | CPython binary invoked by `zvelte dev` (automatically falls back to `python3` / `python`). |
| `[python] execution` | string | `"server"` | Always host-side CPython worker. |
| `[python] rust_acceleration` | bool | `true` | Links optional compiled native shared libraries into Python via `ctypes`. |
| `[toolchain] python` / `rust` | string | `"3.10+"` / `"1.80+"` | Informational version requirements. |

### 4. `[rust]` *(Optional)*
| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Enables optional native Rust module compilation. |
| `executable` | string | `"rustc"` | Native compiler executable (`$RUSTC`, `PATH`, or `~/.cargo/bin/rustc`). |
| `native_modules` | bool | `true` | Enables `rustc --crate-type cdylib` compilation. |

### 5. `[tailwind]` / `[style]`
| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Enables built-in Tailwind CSS compilation (`tailwind = true` alias in `[style]`). |
| `preflight` | bool | `true` | Emits `@layer base` Tailwind reset when Tailwind classes/directives or `[tailwind]` are present. |
| `dark_mode` | string | `"media"` | `"media"` (`@media (prefers-color-scheme: dark)`) or `"class"` (`.dark ...`). |
| `mode` | string | `"auto"` | `"auto"` / `"builtin"` (built-in Rust engine), `"cli"` (runs external `tailwindcss`), or `"off"`. |

### 6. `[dev]`
| Key | Type | Default | Description |
|---|---|---|---|
| `host` | string | `"127.0.0.1"` | Bind IP address. |
| `port` | integer | `3000` | Initial TCP port (`zvelte dev` automatically tries `port..port+9` if busy). |
| `hot_reload` | bool | `true` | Watches `app.toml` and entry `.zvelte` for edits, recompiles `.zvelte/dev`, restarts Python worker, and reloads connected browsers. |
| `open_browser` | bool | `false` | Browser auto-open option flag. |

### 7. `[build]`
| Key | Type | Default | Description |
|---|---|---|---|
| `output` / `out_dir` | string | `"dist"` | Directory where `zvelte build` writes production artifacts (`zvelte clean` removes this directory). |
| `cache` | string | `".zvelte"` | Cache directory removed on `zvelte clean`. |
| `optimization` | string | `"release"` | Build optimization label. |
| `minify` | bool | `false` | Reserved option flag for future minification pass. |
| `source_maps` | bool | `true` | Source map configuration flag. |

> [!NOTE]
> `#` comments inside TOML files work at line starts and unquoted value ends while preserving literal `#` characters inside single- or double-quoted strings (e.g. `name = "Brand #1"`). Legacy `[zig]` / `zig_acceleration` keys are also accepted for backward compatibility.
