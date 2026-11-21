# 📜 Changelog

All notable changes to the **Zvelte** framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
