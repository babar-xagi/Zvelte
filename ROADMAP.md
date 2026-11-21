# 🗺️ Zvelte Technical Roadmap & Phase-by-Phase Execution Plan

> Official technical roadmap for **Zvelte**: A Zig-powered, multi-language reactive web framework with Python and Zig source blocks, a shared component format, and JavaScript/WebAssembly execution backends.

---

## 🎯 Release Milestones

| Milestone | Target Scope | Status |
|---|---|---|
| **`v0.0.1`** | **CLI, Two-File Scaffolding, TOML Loader, SFC Parser, Fine-Grained Reactive JS Codegen, Embedded Dev Server** | **✅ Released** |
| `v0.0.2` | Advanced Expression Parser, Conditional Rendering (`if`/`else`), Keyed List Rendering (`for`/`each`) | ⏳ Next |
| `v0.0.3` | Python Server Worker Process Integration & RPC Bridge | 🗓️ Planned |
| `v0.0.4` | Zig Native C-ABI Shared Library Compilation & Python `ctypes` Linker | 🗓️ Planned |
| `v0.0.5` | Zig Browser WebAssembly Compilation (`wasm32-freestanding`) & JS Host Adapter | 🗓️ Planned |
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

#### Phase 8 — Template Language Essentials
- [ ] Conditional rendering blocks (`{#if condition} ... {/if}`).
- [ ] Keyed list rendering blocks (`{#each items as item (item.id)} ... {/each}`).
- [ ] Form two-way input binding (`bind:value={name}`).

#### Phase 9 — Zig Native Acceleration Modules
- [ ] Auto-compile `<script lang="zig" context="native">` into host `.so` / `.dll`.
- [ ] C ABI memory safety boundary tests.
- [ ] Automatic Python `ctypes` wrapper generator.

#### Phase 10 — CPython Server Worker & Supervisor
- [ ] Spawn and manage CPython worker process from Zig dev server.
- [ ] JSON-RPC / IPC request-response bridge between web client and Python backend.
- [ ] Clean process shutdown and worker restart on file change.

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
