# 🗺️ Zvelte Technical Roadmap & Phase-by-Phase Execution Plan

> Official technical roadmap for **Zvelte**: A Zig-powered, multi-language reactive web framework with Python and Zig source blocks, a shared component format, and JavaScript/WebAssembly execution backends.

---

## 🎯 Release Milestones

| Milestone | Target Scope | Status |
|---|---|---|
| **`v0.0.1`** | **CLI, Two-File Scaffolding, TOML Loader, SFC Parser, Fine-Grained Reactive JS Codegen, Embedded Dev Server** | **✅ Released** |
| **`v0.0.2`** | **Advanced Expression Parser, Conditional Rendering (`if`/`else`), Keyed List Rendering (`for`/`each`), Two-Way Form Input Bindings (`bind:value`)** | **✅ Released** |
| **`v0.0.3`** | **Python Server Worker Process Integration & Line-Buffered JSON-RPC IPC Bridge (Client Async Proxies & Promise-Aware Reactive Updates)** | **✅ Released** |
| **`v0.0.4`** | **Zig Native C-ABI Shared Library Compilation & Python `ctypes` Linker** | **✅ Released** |
| `v0.0.5` | Zig Browser WebAssembly Compilation (`wasm32-freestanding`) & JS Host Adapter | ⏳ Next |
| `v0.1.0` | Production Optimization, Benchmark Suite, Stable Core Release | 🎯 Milestone |

---

## 📋 Comprehensive Phase Roadmap

### Phase 0 — Toolchain Baseline & Environment Verification
- [x] Verify Zig `0.17.1` runtime and compiler target support (`x86_64-linux`).
- [x] Verify CPython `3.14`/`3.15` and `ctypes` interoperability.
- [x] Verify WebAssembly compilation target (`wasm32-freestanding`).
- [x] Initialize native build system with `build.zig` and `build.zig.zon`.

### Phase 1 — CLI Foundation & Strict Two-File Scaffolding
- [x] Implement CLI argument parsing for `new`, `check`, `build`, `dev`, `clean`, `--version`, `--help`.
- [x] Implement project name sanitization and directory traversal protection.
- [x] Enforce directory overwrite protection.
- [x] **Enforce Two-File Guarantee:** `zvelte new <app>` produces strictly `app.toml` and `app.zvelte`.
- [x] Unit test verifying no third initial file is ever created.

### Phase 2 — TOML Configuration Parser & Validation
- [x] Zero-dependency TOML parser (`[project]`, `[app]`, `[toolchain]`, `[python]`, `[zig]`, `[dev]`, `[build]`).
- [x] Typed `AppConfig` struct with strict validation.
- [x] Mode enforcement: `"python"`, `"zig"`, and `"hybrid"`.
- [x] Detailed error diagnostics with line locations.

### Phase 3 — Single-File Component (`.zvelte`) Grammar & Parser
- [x] Raw script block isolation for `<script lang="..." context="...">` avoiding tag collision.
- [x] Typed `<state>` block parser.
- [x] Component element tree parser with attribute and event extraction.
- [x] Dynamic expression (`{...}`) and event binding (`event={...}`) parser.
- [x] Scoped `<style>` extraction.

### Phase 4 — Semantic Analysis & Symbol Validation
- [x] Validate script block languages against project `mode`.
- [x] Build state variable symbol table with uniqueness and type validation.
- [x] Check event handler expressions against declared state variables.

### Phase 5 — Static & Fine-Grained Reactive JavaScript Generator
- [x] HTML shell generation with scoped CSS injection.
- [x] Direct DOM creation (no Virtual DOM overhead).
- [x] Fine-grained reactive dependency tracking (`subscribe(...)` targeting specific text cells).
- [x] Event listener generation (`addEventListener`).

### Phase 6 — Development Server & Build Pipeline
- [x] Embedded zero-dependency HTTP server in Zig (`127.0.0.1:3000`).
- [x] Serving HTML and JS with standard MIME headers.
- [x] Production build pipeline emitting to `dist/index.html` and `dist/app.js`.
- [x] `zvelte clean` restoring workspace cleanly.

---

### Upcoming Phases (Post v0.0.1)

#### Phase 8 — Template Language Essentials (v0.0.2)
- [x] Conditional rendering blocks (`{#if condition} ... {:else} ... {/if}`).
- [x] Keyed list rendering blocks (`{#each items as item (item.id)} ... {/each}`).
- [x] Form two-way input binding (`bind:value={name}` and `bind:checked={flag}`).
- [x] Advanced Expression Parser for state mutations, condition evaluations, and expressions.
- [x] Extended `<state>` data type support for array/slice collections (`[]string`, `list`, `array`).

#### Phase 9 — Zig Native Acceleration Modules (v0.0.4)
- [x] Auto-compile `<script lang="zig" context="native">` into host native shared library (`libnative.so` / `libnative.dylib` / `native.dll`) via `zig build-lib -dynamic -O ReleaseFast`.
- [x] C-ABI function export parser and symbol extractor (`export fn ... callconv(.c)`).
- [x] Automatic Python `ctypes` wrapper generator with typed `argtypes`, `restype`, and numeric argument coercion.
- [x] Python worker integration injecting native Zig C-ABI bindings into Python server `globals()`.
- [x] Client-side asynchronous RPC proxy generation exposing native Zig functions to browser JavaScript (`async function <fn_name>(...args)`).
- [x] CLI `check`, `build`, and `clean` commands updated to handle native shared libraries and artifacts with recursive tree cleanup.

#### Phase 10 — CPython Server Worker & Supervisor (v0.0.3)
- [x] Auto-generate standard-library-only `worker.py` encapsulating user server functions with line-buffered JSON-RPC protocol.
- [x] Spawn and supervise child CPython worker (`python3 -u`) from Zig dev server with non-blocking pipe I/O.
- [x] HTTP JSON-RPC endpoint (`POST /api/rpc`) with CORS `OPTIONS` handling and sub-millisecond local dispatch.
- [x] Client-side asynchronous JavaScript RPC proxy generator (`async function <fn_name>(...args)`) exposing Python functions globally.
- [x] Promise-aware fine-grained text node reactivity (`__setNodeText`) resolving async Python returns directly into the DOM.
- [x] Semantic analyzer validation permitting declared Python server functions in event expressions and bindings.
- [x] Clean child process termination (SIGTERM / terminate) and pipe closure on dev server exit and error unwind.
- [x] Production build pipeline integration emitting `worker.py` in `dist/` with detailed asset size reporting.

#### Phase 11 — Zig WebAssembly Browser Modules
- [ ] Auto-compile `<script lang="zig" context="wasm">` to WebAssembly.
- [ ] Browser JavaScript WASM loader and scalar memory adapter.

#### Phase 12 — WebAssembly DOM Host Runtime
- [ ] Versioned JavaScript host adapter for WebAssembly DOM operations.
- [ ] Batch mutation bridging to minimize JS-WASM boundary crossings.

#### Phase 13 — Production Optimizations & Asset Bundling
- [ ] Minification and tree-shaking for generated JavaScript.
- [ ] Content-hashed asset naming for caching.

#### Phase 14 — Security Hardening & Penetration Testing
- [ ] Strict text node escaping preventing XSS injection.
- [ ] Input payload validation across Python and Zig boundaries.

#### Phase 15 — Benchmarking Suite & 0.1.0 Stable Release
- [ ] Performance benchmarks comparing initial render latency, DOM update speed, and memory usage against vanilla JS and modern frameworks.
- [ ] Documentation and tutorial finalization.
