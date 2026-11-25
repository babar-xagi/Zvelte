# 🗺️ Zvelte Technical Roadmap

> Official technical roadmap for **Zvelte**: A Python reactive web framework powered by a zero-dependency Rust compiler and runtime engine.

---

## 🎯 Release Milestones

| Milestone | Target Scope | Status |
|---|---|---|
| **`v0.0.1`** | **CLI, Two-File Scaffolding, TOML Loader, SFC Parser, Reactive JS Codegen, Dev Server** | **✅ Released** |
| **`v0.0.2`** | **Conditional Rendering (`{#if}`/`{:else}`), List Rendering (`{#each}`), Form Bindings (`bind:value`, `bind:checked`)** | **✅ Released** |
| **`v0.0.3`** | **Python Server Worker Supervisor & Line-Buffered JSON-RPC Bridge (`POST /api/rpc`)** | **✅ Released** |
| **`v0.0.4`** | **Native C-ABI Shared Library Compilation, `ctypes` Linker, Svelte Ergonomics & Port Fallback** | **✅ Released** |
| **`v0.0.5`** | **Full Rust Core Rewrite (`std`-only), Python-First Scaffold & `examples/03-python-api`** | **✅ Released** |
| **`v0.0.6`** | **Live Hot Reload Watcher, Scoped Reactivity Cleanup, Server Secret Isolation, Full Test Suite** | **✅ Released** |
| `v0.1.0` | Stable Core Release: Client JS State Transform, Minification, XSS Audit, Multi-Component Imports | 🎯 Next |

---

## 📋 Completed Core Engine Phases

### Phase 1 — Rust Core CLI & Strict Two-File Scaffolding
- [x] Zero-dependency Rust CLI (`zvelte` & `zvelte-rs`) built exclusively on Rust `std`.
- [x] Commands: `new`, `check`, `build`, `dev`, `clean`, `--version`, `--help`.
- [x] Strict two-file guarantee (`app.toml` + `app.zvelte`), project name validation, overwrite protection.

### Phase 2 — TOML Configuration & Multi-Mode Validation
- [x] Quote-aware zero-dep TOML parser supporting `[project]`, `[app]`, `[toolchain]`, `[python]`, `[rust]`, `[dev]`, `[build]`.
- [x] Configurable build output (`[build] output`), Python binary (`[python] executable`), and dev hot reload (`[dev] hot_reload`).

### Phase 3 — `.zvelte` Parser & Semantic Analyzer
- [x] Raw `<script>` parsing with Svelte-style defaults (`<script>` -> JS client, `<script lang="python">` -> Python server, `<script lang="rust">` -> Rust native).
- [x] HTML comment skipping (`<!-- ... -->`) and quote-aware brace expression scanning.
- [x] Literal-only auto-state extraction so Python server calls (`os.getenv(...)`) stay strictly off the client bundle.
- [x] `{#if}`/`{:else}`, `{#each}`, `bind:value`, `bind:checked`, and typed `<state>` parsing.

### Phase 4 — Scoped Reactive JS Codegen & CPython RPC Runtime
- [x] Direct DOM creation with hierarchical subscriber scope disposal (`__clear_scope`) — zero Virtual DOM overhead.
- [x] Supervised CPython child worker (`worker.py`) over line-buffered `stdin`/`stdout` JSON-RPC (`POST /api/rpc`).
- [x] Promise-aware DOM text resolution (`__setNodeText`) and Windows CRLF-safe string escaping.
- [x] Optional native Rust C-ABI compilation (`rustc --crate-type cdylib -O`) + `ctypes` bindings in `hybrid` / `rust` modes.

### Phase 5 — Cross-Platform DevServer & Live Hot Reload
- [x] Cross-platform `std::net::TcpListener` HTTP server with automatic fallback across `port..port+9`.
- [x] File-watcher hot reload monitoring `app.toml` and entry `.zvelte`, rebuilding `.zvelte/dev`, restarting `worker.py`, and notifying browsers via `/__zvelte_version`.

---

## 🔮 Upcoming Work (`v0.1.0`)

- [ ] **Compiled Client `<script>` Reactivity:** Rewrite client `<script>` variable mutations (`count++` inside user helper functions) to mutate `state.count`.
- [ ] **JavaScript & CSS Minification:** Implement production minifier wired to `[build] minify = true` and content-hashed asset filenames.
- [ ] **HTML Attribute & Text Interpolation Validation:** Extend semantic analyzer validation to `{expr}` dynamic text and attribute bindings.
- [ ] **Multi-Component Imports:** Support importing child `.zvelte` components.
- [ ] **WebAssembly Target (`renderer = "wasm"` / `context="wasm"`):** Compile browser compute modules to `wasm32` when enabled.
- [ ] **Automated XSS Hardening:** Ensure dynamic HTML attributes and URLs are sanitized before DOM insertion.
