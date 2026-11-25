# 📜 Changelog

All notable changes to the **Zvelte** framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.0.6] - 2026-11-25

### 🛡️ Security Hardening, Live Hot Reload, Parser Fixes & Comprehensive Test Suite

#### 🐛 Bug Fixes
- **Eliminated Server Expression Leakage & Browser Crash in `04-ai-chatbot`:** Fixed `extract_script_state_variables` in [parser.rs](src/source/parser.rs) so only safe JSON/JS-compatible literal values (numbers, booleans, `null`/`None`, quoted strings, literal arrays/dicts) are auto-extracted into client `state`. Non-literal server expressions such as `GEMINI_KEY = os.getenv(...)` stay strictly on the Python server and no longer leak or throw `ReferenceError` in `app.js`.
- **HTML Comment Support:** Added `skip_html_comment` across root, element, `{#if}`, and `{#each}` parsing loops so `<!-- ... -->` comments parse cleanly without `MismatchedTag` or `UnexpectedEOF`.
- **String Literal Preservation & Escaping in JS Codegen:**
  - Updated `JsCodegen::format_expr` to skip quoted string literals (`"..."`, `'...'`, `` `...` ``) verbatim instead of rewriting identifiers inside strings as `state.<ident>`.
  - Added full `\r` (`\r\n` CRLF) and attribute escaping in generated JS strings and HTML `<title>`, preventing Windows line-ending syntax errors.
  - Quoted brace-expression scanner (`read_braced_expression`) now ignores braces inside strings (e.g., `click={msg = "a } b"}`).
- **TOML Quoted `#` Handling:** Updated [toml_parser.rs](src/config/toml_parser.rs) so `#` characters inside single- or double-quoted TOML values are preserved rather than stripped as comments, and added `[build] out_dir` alias support.
- **Scoped `{#if}` / `{#each}` Subscriber Cleanup:** Replaced unbounded subscriber growth with hierarchical scoped tracking (`subscribe`, `__clear_scope`) and snapshot iteration in `update()`, preventing stale DOM callbacks on re-render.

#### ✨ Implemented Claims & Improvements
- **Live Hot Reload File Watcher (`zvelte dev`):** [server.rs](src/devserver/server.rs) now monitors `app.toml` and the entry `.zvelte` file for modification changes when `hot_reload = true`, recompiling `.zvelte/dev`, restarting the supervised Python worker, exposing `GET /__zvelte_version`, and auto-refreshing connected browsers.
- **Wired `[build] output` & `[python] executable`:** `zvelte build` and `zvelte clean` respect `[build] output` and `[build] cache`, and `DevServer` respects `[python] executable`.
- **HTTP Request Hardening:** Added loop-until-headers-complete reading, case-insensitive `Content-Length` parsing, and blocking client stream reset in `DevServer`.
- **Documentation Sync:** Updated `README.md`, `ROADMAP.md`, all `docs/*.md` files, and `examples/01..03` to match the Rust + Python v0.0.6 architecture.

#### 🧪 Test Suite
- Added comprehensive integration & unit tests in [tests/comprehensive_tests.rs](tests/comprehensive_tests.rs) plus library unit tests in [src/lib.rs](src/lib.rs), covering TOML, parser, semantic checks, JS/Python codegen, CRLF, string preservation, native C-ABI, scaffold contract, CLI workflow, live DevServer HTTP + Python RPC, and live Hot Reload.

---

## [0.0.5] - 2026-11-23

### 🦀 Migration to Zero-Dependency Rust Core Engine & Python-First Developer Experience

#### ✨ Core Features
- **Full Rust Rewrite (`std`-only):** Replaced the Zig compiler implementation with a pure Rust (`edition = "2024"`) compiler, parser, semantic analyzer, JS/Python code generator, and cross-platform HTTP dev server compiled cleanly under ~500 KB (`opt-level = "z"`, LTO, stripped).
- **Python-First Scaffolding:** `zvelte new <app>` defaults to `mode = "python"` generating pure Python server logic + Svelte reactive markup, while still supporting optional native Rust C-ABI acceleration (`<script lang="rust">`) in `hybrid`/`rust` modes.
- **Dual Binary Targets:** Emits both `zvelte` and `zvelte-rs`.
- **Benchmark Suite Comparison:** Recorded Zig vs. Rust empirical metrics in [comparison.md](comparison.md).

---

## [0.0.4] - 2026-11-23

### ⚡ Native C-ABI Shared Library Compilation & Python `ctypes` Linker (Phase 9)
- Added native C-ABI compilation (`libnative.so` / `libnative.dylib` / `native.dll`), automated `ctypes` bindings in `worker.py`, Svelte-style optional script attributes, automatic top-level state inference, clean SIGINT handling, cross-platform sockets, and automatic dev port fallback.

## [0.0.3] - 2026-11-22

### ⚡ Python Server Worker Process Integration & JSON-RPC Bridge (Phase 10)
- Added embedded CPython worker supervisor (`worker.py`), line-buffered stdio JSON-RPC bridge, `POST /api/rpc`, client async proxies, and Promise-aware DOM text binding.

## [0.0.2] - 2026-11-22

### ⚡ Template Language Essentials & Advanced Reactivity (Phase 8)
- Added conditional blocks (`{#if}`/`{:else}`), list blocks (`{#each}`), and two-way form bindings (`bind:value`, `bind:checked`).

## [0.0.1] - 2026-11-21

### 🚀 Initial Framework Release
- Deterministic two-file scaffolding (`app.toml` + `app.zvelte`), `.zvelte` parser, TOML parser, semantic analyzer, fine-grained JS codegen, dev server, and `check`/`build`/`clean` CLI.
