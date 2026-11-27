# 📜 Changelog

All notable changes to the **Zvelte** framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.0.9] - 2026-11-27

### ⚛️ Svelte-Parity Reactivity, 🐍 Async Python RPC & 🧹 Full Audit Hardening
- **Reactive `$:` Derived Declarations ([parser.rs](src/source/parser.rs), [js_codegen.rs](src/compiler/js_codegen.rs)):** Added parsing and automatic recomputation (`__recompute_derived()`) inside `update()` (with re-entrancy guard `__updating`) for `$: doubled = count * 2;`-style reactive statements.
- **Compiled Client `<script>` Reactivity:** Hoisted client `<script>` literal state declarations cleanly, rewritten non-shadowed state references inside helper functions/arrow functions to `state.*`, and auto-triggered `update()` after state-mutating helper execution.
- **Python `async def` Coroutine Execution & Direct Event `await` ([codegen.rs](src/compiler/codegen.rs), [js_codegen.rs](src/compiler/js_codegen.rs)):** Added `asyncio.run(...)` detection (`inspect.iscoroutine(res)`) inside `worker.py` so Python `async def` functions execute cleanly over JSON-RPC, and automatically `await` unchained RPC calls inside DOM event handlers so assigned `state.*` holds the resolved value (not a pending `Promise`) while keeping `.then(...)` chains intact.
- **Standard Svelte Event Syntax & Modifiers:** Support `on:click`, `onclick`, and `click` (across `click`, `input`, `change`, `submit`, `keydown`, `keyup`, `focus`, `blur`, `mouseenter`, `mouseleave`), bare function callbacks (`on:click={increment}`), arrow closures (`onclick={() => ...}`), and modifiers (`|preventDefault`, `|stopPropagation`, `|once`).
- **Control Flow & Style Upgrades:** Added `{:else if ...}` branches, `{#each list as item, idx (key)}` comma index variable extraction (with scoped local protection so `item`/`idx` are never mis-prefixed with `state.`), `<style lang="css">` attribute handling + multiple `<style>` block merging, and complete HTML5 void elements.
- **Full Template Semantic Checks & Working `[build] minify` ([analyzer.rs](src/semantic/analyzer.rs)):** Extended semantic validation cleanly to `{expr}` dynamic text and attribute bindings (supporting backtick template interpolations and `(a, b) =>` arrow parameters), added `[python] version` TOML key parsing, and wired `[build] minify = true` in production builds.
- **Test Suite:** Expanded to **31 passing tests** (`2` library unit tests + `29` integration tests).

---

## [0.0.8] - 2026-11-25

### ⚡ Full Built-In Zero-Dependency UnoCSS Engine

#### ✨ Core Features
- **Pure Rust UnoCSS Compiler ([unocss.rs](src/compiler/unocss.rs)):** Built-in zero-Node UnoCSS engine sharing core palette/variant primitives with [tailwind.rs](src/compiler/tailwind.rs) for single-pass CSS compilation (`PREFLIGHT_CSS` and `@keyframes` emitted once).
- **Attributify Mode:** Supports both valueless boolean utility attributes (`<header flex items-center pb-6 bg-slate-900/80>`) -> `[attr]` CSS selectors and grouped attributes (`bg="slate-950/60 hover:slate-900"`, `p="x-6 y-8"`, `border="~ slate-800 rounded-xl"`, `hover="bg-sky-500 text-white"`) -> `[attr~="val"]` selectors while guarding standard HTML, `aria-*`, `data-*`, SVG presentation, and `<input size>` attributes.
- **Variant Group Expansion (`variant:(a b c)`):** Expands single and nested variant groups (`dark:(bg-slate-900/90 hover:(border-sky-500/60 scale-105))`) across BOTH compiled CSS rules AND runtime JS `setAttribute` DOM generation (including dynamic ternaries/template expressions), while leaving arbitrary CSS parens (`bg-[rgb(1,2,3)]`, `w-[calc(100%-2rem)]`) untouched.
- **TOML Shortcuts (`[unocss.shortcuts]`):** Supports composable and recursive shortcuts with cycle detection (`pill`, `uno-card`, `uno-btn`, `uno-btn-primary`) configured in `app.toml`, emitted before utility layers so explicit overrides always win.
- **Preset-Uno Shorthands & Variants:** Adds `flex-center`, `inline-flex-center`, `flex-col-center`, `flex-between`, `flex-around`, `grid-center`, `pa-*`, `c-*`, `op-*`, `fw-*`, `lh-*`, `ws-*`, `ta-*`, `fs-*`, `cur-*`, `of-*`, `pos-*`, `rd*`, `b*`/`bx*`/`by*`/`bt*`/`br*`/`bb*`/`bl*`, `ma`/`mxa`/`mya`, `un-` prefix, trailing `!` important modifier (`p-6!`), `lt-sm`..`lt-2xl`, `hocus:`, `children:`, `all:`, and `not-*` variants.
- **CSS Directives & Functions:** Expands `--at-apply:`, `@apply`, `@screen sm..2xl` / `@screen lt-*`, `@unocss;`, `@unocss preflights;`, `@import "unocss";`, and `theme('colors.sky.400')` / `theme('spacing.6')` inside `<style>`, plus optional `mode = "cli"` external `unocss` binary support.
- **Parser Slashed Attribute Fix ([parser.rs](src/source/parser.rs)):** Preserves slashes `/` inside valueless attributify attribute names such as `bg-slate-900/80` and `w-1/2` while cleanly stopping at self-closing `/>`.
- **New Showcase Example:** Added [`examples/06-unocss-showcase`](examples/06-unocss-showcase/) featuring Attributify Mode, nested variant groups, shortcuts, `--at-apply:`, `@screen`, `theme()`, and Python JSON-RPC telemetry.

#### 🧪 Test Suite
- Expanded test suite to **29 passing tests** (`cargo test`: 2 lib unit tests + 27 integration tests), adding verification for `[unocss]` / `[unocss.shortcuts]` TOML parsing, nested variant groups + arbitrary parens safety, attributify boolean/grouped selectors, shortcut layering order, JS DOM expansion, HTML/SVG attribute guards, and all 6 repository examples.

---

## [0.0.7] - 2026-11-25

### 🎨 Full Built-In Zero-Dependency Tailwind CSS Engine

#### ✨ Core Features
- **Pure Rust Tailwind CSS Compiler ([tailwind.rs](src/compiler/tailwind.rs)):** Built-in utility CSS compiler requiring zero Node.js, zero npm, and zero `node_modules` (sub-4ms full component + 98-utility dashboard compilation).
- **Complete Tailwind Color Palette & Opacity:** All 22 standard Tailwind color families (`50`–`950`), `black`/`white`/`transparent`/`current`, `/0`–`/100` opacity modifiers (`bg-sky-500/80`), arbitrary hex/rgb colors (`bg-[#1e293b]/80`), and CSS gradients (`bg-gradient-to-*`, `from-*`, `via-*`, `to-*`).
- **Full Utility Coverage & Variants:** Layout, flexbox, grid, spacing (with negatives & `ps`/`pe`/`ms`/`me`), sizing (fractions & `size-*`), typography (`line-clamp-*`, tracking, leading), borders/rounded/rings, shadows, backdrop blur, transitions, `@keyframes` animations (`animate-spin/ping/pulse/bounce`), important (`!`), and arbitrary properties (`[mask-type:alpha]`).
- **Stackable Modifiers:** Responsive breakpoints (`sm:`..`2xl:`, `max-*:`), `dark:` (`media` & `.dark` class modes), pseudo-classes (`hover:`, `focus:`, `focus-visible:`, `disabled:`, `first:`, `odd:`, etc.), `group-*` / `peer-*`, and pseudo-elements (`before:`, `after:`, `placeholder:`, `selection:`).
- **`@apply` & Directives Support:** Expands `@apply ...;` rules inside `<style>` alongside `@tailwind base/components/utilities` and `@import "tailwindcss"`.
- **Dynamic Class Expression Scanning:** Extracts utilities from ternary `class={cond ? "..." : "..."}` and backtick template expressions `` class={`... ${cond ? '...' : '...'}`} `` cleanly (`format_expr` now recursively formats `${...}` inside template literals).
- **Zero-Overhead Auto Preflight & `[tailwind]` Config:** Configured via `[tailwind]` / `[style]` in `app.toml` (`enabled`, `preflight`, `dark_mode`, `mode = "auto" | "builtin" | "cli" | "off"`), automatically skipping Preflight overhead when no Tailwind classes are used unless `[tailwind]` is declared explicitly.
- **New Showcase Example:** Added [`examples/05-tailwind-dashboard`](examples/05-tailwind-dashboard/) demonstrating theme toggling, responsive cards, gradients, `@apply`, and Python RPC.

#### 🧪 Test Suite
- Expanded integration & unit test suite to **26 passing tests** (`cargo test`: 2 lib unit tests + 24 integration tests), verifying Tailwind utilities, stacked variants, gradients, `@apply`, backtick dynamic classes, zero-overhead mode, and all 5 example apps.

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
