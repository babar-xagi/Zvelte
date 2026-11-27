# 🗺️ Zvelte Technical Roadmap

> Official technical roadmap for **Zvelte** — a **Svelte-inspired Python reactive web framework powered by a zero-dependency Rust compiler & runtime engine**.
> Our goal: give Python developers the ergonomic reactive authoring experience of Svelte (`.zvelte` SFCs, `$:` derived state, scoped DOM updates, Tailwind & UnoCSS, native Python/Rust integration) with a single tiny native binary.

---

## 🎯 Release Milestones

| Version | Milestone Scope | Status |
|---|---|---|
| **`v0.0.1`** | CLI (`new`, `check`, `build`, `dev`, `clean`), Deterministic 2-File Scaffold, TOML Loader, SFC Parser | **✅ Released** |
| **`v0.0.2`** | Conditional Rendering (`{#if}`/`{:else}`), Lists (`{#each}`), Two-Way Bindings (`bind:value`, `bind:checked`) | **✅ Released** |
| **`v0.0.3`** | Supervised CPython Server Worker (`worker.py`) & JSON-RPC Bridge (`POST /api/rpc`) | **✅ Released** |
| **`v0.0.4`** | Native C-ABI Shared Library (`cdylib`) Compilation, Python `ctypes` Linker & Dev Port Fallback | **✅ Released** |
| **`v0.0.5`** | Full Zero-Dependency Rust (`edition = "2024"`, `std`-only) Rewrite & Python-First Scaffold | **✅ Released** |
| **`v0.0.6`** | Live Hot Reload File Watcher, Hierarchical Scoped Cleanup, Server Secret Isolation & Test Suite | **✅ Released** |
| **`v0.0.7`** | Built-In Zero-Node Tailwind CSS Engine (`tailwind.rs`), Variants, `@apply`, Gradients & Dynamic Classes | **✅ Released** |
| **`v0.0.8`** | Built-In Zero-Node UnoCSS Engine (`unocss.rs`), Attributify Mode, Nested Variant Groups, Shortcuts & `--at-apply` | **✅ Released** |
| **`v0.0.9`** | Svelte-Parity Reactivity (`$:`, Client `<script>`, `on:event\|modifiers`, `{:else if}`, `{#each item, idx}`, Python `async def`, `await` RPC, `minify`) | **✅ Released** |
| **`v0.1.0`** | Component Imports (`import`), Props & `<slot />`, Component-Scoped CSS & Source Maps | **🎯 Next** |
| **`v0.2.0`** | `{@html}`, `{#await}`, Lifecycle Hooks (`onMount`), `transition:*`, Python SSE Streaming & `wasm32` Target | **🔭 Planned** |

---

## ✅ What We Have Done

### 1. 🦀 Zero-Dependency Rust Core Compiler & CLI
- [x] Standalone `std`-only Rust engine (`zvelte` & `zvelte-rs`, ~699 KB release binary).
- [x] CLI workflow: `zvelte new`, `check`, `build`, `dev`, `clean`, `--version`, `--help`.
- [x] Deterministic **Two-File Contract**: `zvelte new <app>` creates strictly `app.toml` and `app.zvelte` cleanly.
- [x] Custom `[build] output`, `[build] cache`, `[build] minify = true`, and `[python] executable` wiring.

### 2. ⚡ Svelte-Style Component Compiler & Fine-Grained DOM Reactivity
- [x] `.zvelte` parser supporting `<script>`, `<script lang="python">`, `<script lang="rust">`, `<state>`, `<style ...>`, and HTML comments (`<!-- ... -->`).
- [x] Client `<script>` reactive hoisting & automatic `state.` rewriting across functions and arrow helpers, avoiding shadowing bugs with local/parameter scope tracking.
- [x] `$: derived = expression` Svelte reactive computed declarations recalculated inside `update()` before subscribers fire.
- [x] Fine-grained virtual-DOM-free updates with hierarchical scope cleanup (`__clear_scope`) preventing subscriber memory leaks inside `{#if}`, `{:else if}`, `{:else}`, and `{#each list as item, idx (key)}`.
- [x] Standard Svelte & HTML event syntax (`click={...}`, `on:click={...}`, `onclick={...}`), bare function refs, arrow closures `(e) => ...`, and modifiers (`|preventDefault`, `|stopPropagation`, `|once`).
- [x] Full semantic validation covering mode rules, state types, bindings, loops, events, AND template `{expr}` / dynamic attribute expressions.

### 3. 🐍 Python Server Runtime, Security & Optional Native C-ABI
- [x] Safe literal-only state hoisting so Python server secrets (`os.getenv(...)`, API keys) stay isolated inside CPython `worker.py` and never leak into browser `app.js`.
- [x] Supervised CPython child worker over line-buffered `stdin`/`stdout` JSON-RPC (`POST /api/rpc`), including automatic `asyncio.run(...)` support for Python `async def` coroutines.
- [x] Client RPC proxies (`async function`) automatically awaited cleanly when assigned inside event handlers (`summary = analyze(...)`), while preserving explicit `.then(...)` chains.
- [x] Live Hot Reload DevServer (`zvelte dev`) with automatic port probing (`port..port+9`), file mtime watcher, worker restart, and `/__zvelte_version` browser refresh.
- [x] Optional `<script lang="rust">` native `cdylib` compilation via `rustc` linked automatically into Python `worker.py` via `ctypes`.

### 4. 🎨 Zero-Node Tailwind CSS + UnoCSS Engines
- [x] Full built-in Tailwind CSS engine (22 color families, `/0`–`/100` opacity, arbitrary `[...]` values, gradients, keyframes, `@apply`, responsive/dark/hover/group/peer variants).
- [x] Full built-in UnoCSS engine (boolean & grouped Attributify `[attr]` / `[attr~="val"]`, nested `variant:(...)` expansion across CSS and DOM `setAttribute`, TOML `[unocss.shortcuts]`, preset shorthands, `--at-apply:`, `@screen`, `theme()`, and `un-` / `p-6!`).
- [x] Zero CSS overhead when styling utilities are not used and not explicitly enabled.

---

## 🔭 What Is Remaining (Roadmap to `v0.1.0` & `v1.0.0`)

### Priority 1 — Component Composition (`v0.1.0`)
- [ ] **Multi-File `.zvelte` Imports:** Support importing reusable child components (`import Card from './Card.zvelte'`).
- [ ] **Component Props & Two-Way Bindable Props:** Support `export let title = "..."` / Svelte 5 `$props()` and `<Card bind:value={...} />`.
- [ ] **Slots (`<slot />` & Named Slots):** Parent-to-child markup composition inside imported components.
- [ ] **Component-Scoped CSS:** Automatically scope `<style>` rules per component with hashed attributes (e.g., `[data-zvelte-abc123]`) and `:global(...)` opt-out.

### Priority 2 — Rich Svelte Directives & Security (`v0.1.0`–`v0.2.0`)
- [ ] **Async Control & Raw HTML Blocks:** `{#await promise}...{:then res}...{:catch err}{/await}` and `{@html sanitized_expr}`.
- [ ] **Lifecycle Hooks & Effects:** `onMount(() => ...)`, `onDestroy(() => ...)`, and statement-level `$: { sideEffect(); }` / `$effect(...)`.
- [ ] **Actions & Built-in Transitions:** `use:action` element lifecycle actions and `transition:fade` / `transition:fly`.
- [ ] **XSS URL / HTML Hardening & Production Polish:** URL protocol sanitization (`href`/`src`), content-hashed assets, full JS/CSS AST minification, and source maps (`[build] source_maps = true`).

### Priority 3 — Python Streaming & WebAssembly (`v0.2.0+`)
- [ ] **Python Streaming RPC (SSE / WebSocket):** Token-by-token async generator streaming from `<script lang="python">` (`yield`) for real-time AI/LLM responses and live feeds.
- [ ] **Client-Side Router & Multi-Page Builds:** Built-in URL routing (`navigate()`, dynamic `[slug]` routes) and SSR/SSG pre-rendering.
- [ ] **WebAssembly Target (`renderer = "wasm"` / `context="wasm"`):** Compile client reactivity and compute functions directly to `wasm32`.
- [ ] **Dev Worker Graceful Shutdown & PyPI/Cargo Distribution:** Clean OS signal handling on Windows/Unix + prebuilt GitHub binaries.
