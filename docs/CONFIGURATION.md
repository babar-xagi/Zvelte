# ⚙️ `app.toml` Configuration Reference

> The complete specification and reference for `app.toml`, the single authoritative configuration file for Zvelte applications.

---

## 📄 Complete Example Configuration

```toml
[project]
name = "my-app"
version = "0.1.0"
entry = "app.zvelte"

[app]
target = "web"
mode = "hybrid"              # Options: "python", "zig", "hybrid"
renderer = "js"               # Options: "js", "wasm", "auto"
strict = true

[toolchain]
zig = "0.17.1"
python = "3.15"

[python]
enabled = true
executable = "python3"
execution = "server"          # CPython host execution; not browser-side
zig_acceleration = true

[zig]
enabled = true
executable = "zig"
native_modules = true
wasm_modules = true

[dev]
host = "127.0.0.1"
port = 3000
hot_reload = true
open_browser = false

[build]
output = "dist"
cache = ".zvelte"
optimization = "debug"        # Options: "debug", "release-safe", "release-fast"
minify = false
source_maps = true
```

---

## 🔍 Section Breakdown

### 1. `[project]`
Metadata defining the application identity and primary entrypoint.

| Key | Type | Default | Description |
|---|---|---|---|
| `name` | string | **Required** | Project name (used for HTML title and artifact tagging). |
| `version` | string | `"0.1.0"` | Semantic version string of the application. |
| `entry` | string | `"app.zvelte"` | Primary component entry file. |

---

### 2. `[app]`
Core runtime mode and compilation behavior.

| Key | Type | Default | Description |
|---|---|---|---|
| `target` | string | `"web"` | Compilation target platform. |
| `mode` | string | `"hybrid"` | Permitted language block mode: `"python"`, `"zig"`, or `"hybrid"`. |
| `renderer` | string | `"js"` | Client execution backend: `"js"` (default) or `"wasm"`. |
| `strict` | bool | `true` | When `true`, unknown keys and type violations cause immediate failure. |

#### Language Mode Rules:
- **`mode = "python"`:** Disallows Zig script blocks (`native` and `wasm`). Fails with `ZigBlockNotAllowedInPythonMode` if a Zig block is detected.
- **`mode = "zig"`:** Disallows Python server script blocks. Fails with `PythonBlockNotAllowedInZigMode` if a Python block is detected.
- **`mode = "hybrid"`:** Permits both Python and Zig blocks simultaneously.

---

### 3. `[toolchain]`
Specifies compiler and language versions required by the project.

| Key | Type | Default | Description |
|---|---|---|---|
| `zig` | string | `"0.17.1"` | Required Zig compiler version. |
| `python` | string | `"3.15"` | Target Python version. |

---

### 4. `[python]`
Configuration for the Python server supervisor and acceleration bindings.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Enables or disables Python server block parsing and execution. |
| `executable` | string | `"python3"` | Path or command name for the CPython binary. |
| `execution` | string | `"server"` | Execution environment (always `"server"` in v0.1). |
| `zig_acceleration` | bool | `true` | When `true`, automatically links native Zig shared library functions to Python. |

---

### 5. `[zig]`
Settings for native compilation and WebAssembly module building.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Enables or disables Zig compilation. |
| `executable` | string | `"zig"` | Path or command name for the Zig executable. |
| `native_modules` | bool | `true` | Enables host shared library compilation for server acceleration. |
| `wasm_modules` | bool | `true` | Enables `wasm32-freestanding` compilation for browser compute modules. |

---

### 6. `[dev]`
Configuration for the embedded development server.

| Key | Type | Default | Description |
|---|---|---|---|
| `host` | string | `"127.0.0.1"` | Bind IP address for local HTTP development. |
| `port` | integer | `3000` | TCP port number (must be between 1 and 65535). |
| `hot_reload` | bool | `true` | Watches `app.toml` and `app.zvelte` for live browser updates. |
| `open_browser` | bool | `false` | Automatically opens default browser on server start. |

---

### 7. `[build]`
Settings for production code generation and asset bundling.

| Key | Type | Default | Description |
|---|---|---|---|
| `output` | string | `"dist"` | Directory where production artifacts are placed. |
| `cache` | string | `".zvelte"` | Directory for intermediate build artifacts and compiler cache. |
| `optimization` | string | `"debug"` | Optimization level: `"debug"`, `"release-safe"`, or `"release-fast"`. |
| `minify` | bool | `false` | Minifies generated JavaScript and HTML in production. |
| `source_maps` | bool | `true` | Generates source maps for development debugging. |
