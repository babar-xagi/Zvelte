# 📜 Changelog

All notable changes to the **Zvelte** framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.0.2] - 2026-11-22

### ⚡ Template Language Essentials & Advanced Reactivity (Phase 8)

#### ✨ Core Features
- **Conditional Rendering Blocks (`{#if}` / `{:else}` / `{/if}`):** Supports conditional template rendering with `display: contents` wrapper containers and fine-grained subscriber recreation without Virtual DOM diffing.
- **Keyed List Rendering Blocks (`{#each items as item}` / `{#each items as item (key)}`):** Dynamic loop iteration over reactive array collections in state, generating individual element templates with lexical scope preservation.
- **Two-Way Form Input Bindings (`bind:value` / `bind:checked`):** Direct two-way synchronization on `<input>` elements with cursor stabilization checks (`el.value !== state.x`) preventing input jumping.
- **Advanced Expression Parser:** Automatic lexical scoping for complex expressions, mathematical operations, array mutations (`todos.push(...)`), and ternary conditions, cleanly mapping state variables to reactive state properties.
- **Extended State Types:** Added support for array/slice types (`[]string`, `[]i32`, `list`, `array`) in `<state>` blocks.
- **Enhanced Example Application:** Updated `examples/02-todo-app` into a fully interactive task manager demonstrating dynamic list rendering, conditional completion banners, and two-way form input bindings.

#### 🧪 Test Suite
- New unit and semantic tests for template language essentials:
  - Conditional if/else block parsing and AST validation.
  - Keyed and unkeyed each block parsing and syntax error rejection.
  - Two-way input binding parsing and unknown binding state variable rejection.
  - Code generation verification for dynamic if/else wrappers, each loop iterators, and input event bindings.

---

## [0.0.1] - 2026-11-21

### 🚀 Initial Framework Release

#### ✨ Core Features
- **Deterministic Two-File Scaffolding:** `zvelte new <app>` creates strictly `app.toml` and `app.zvelte`. Verified with zero extraneous files.
- **Unified Single-File Component (`.zvelte`) Parser:** Parses multi-language script blocks (`python` server, `zig` native, `zig` wasm), typed `<state>` declarations, markup element trees, event handlers (`click={...}`), dynamic expressions (`{...}`), and scoped `<style>`.
- **Zero-Dependency TOML Parser:** Pure Zig parser supporting section tables, string literals with escape sequences, booleans, integers, floats, and comments.
- **Typed Configuration Model (`AppConfig`):** Validates project mode (`python`, `zig`, `hybrid`), execution targets, port constraints, and toolchain versions.
- **Semantic Analyzer:** Symbol verification preventing duplicate state names, verifying event expression mutation targets, and enforcing project language mode constraints.
- **Fine-Grained Reactive JavaScript Generator:** Direct DOM manipulation generator with subscriber callbacks targeting specific text nodes without Virtual DOM reconciliation overhead.
- **Embedded Development Server:** High-performance native HTTP server in Zig listening on `127.0.0.1:3000` with correct MIME types for HTML and JavaScript.
- **Production Build Pipeline:** `zvelte build` emits clean, standalone `dist/index.html` and `dist/app.js`.
- **Clean Command:** `zvelte clean` safely deletes generated cache (`.zvelte/`) and build outputs (`dist/`) while strictly preserving user source files.

#### 🧪 Test Suite
- Comprehensive test coverage across all subsystems:
  - TOML parser conformance.
  - Configuration loader and validation.
  - Two-file scaffold creation and overwrite protection.
  - Strict two-file invariant iteration test.
  - Static HTML parser.
  - Multi-language hybrid counter component parser.
  - Semantic analysis and mode rejection.
  - Static and fine-grained reactive JavaScript codegen.
  - Embedded dev server socket binding.
