# ⚡ Zvelte Performance & Benchmarks

> Empirical benchmark measurements of the **Zvelte `v0.0.9`** Rust compiler + Svelte reactivity, Tailwind CSS & UnoCSS engines (`opt-level = "z"`, LTO, stripped) across the repository examples.

---

## 📊 Benchmark Summary (`v0.0.9` — Rust Core + Svelte Reactivity + Tailwind & UnoCSS)

Measured on **Linux x86_64** using **Python 3.14.4** and **`target/release/zvelte` (`716,080` bytes / `699.3 KB`)**:

| Project | `zvelte check` (20-run mean) | `zvelte build` (20-run mean) | `dist/index.html` | `dist/app.js` |
|---|---|---|---|---|
| [`examples/01-counter`](../examples/01-counter/) | **2.41 ms** | **2.99 ms** | 1,918 B | 4,862 B |
| [`examples/02-todo-app`](../examples/02-todo-app/) | **2.37 ms** | **3.31 ms** | 2,265 B | 9,406 B |
| [`examples/03-python-api`](../examples/03-python-api/) *(`[build] minify = true`)* | **2.49 ms** | **3.66 ms** | 1,369 B | 8,820 B |
| [`examples/04-ai-chatbot`](../examples/04-ai-chatbot/) | **3.24 ms** | **5.09 ms** | 3,289 B | 16,974 B |
| [`examples/05-tailwind-dashboard`](../examples/05-tailwind-dashboard/) *(98 Tailwind utilities + `@apply`)* | **2.28 ms** | **4.09 ms** | 7,298 B | 15,893 B |
| [`examples/06-unocss-showcase`](../examples/06-unocss-showcase/) *(95 UnoCSS rules + 13 utilities + Attributify + Shortcuts)* | **2.63 ms** | **5.24 ms** | 7,465 B | 19,977 B |

> For historical Zig vs. Rust architecture benchmarks from the migration decision, see [`comparison.md`](../comparison.md).

---

## 🥊 Architecture Comparison: Fine-Grained Scoped DOM vs Virtual DOM

| Architectural Feature | Traditional Virtual DOM + PostCSS / Node UnoCSS | Zvelte Rust + Tailwind / UnoCSS Engine |
|---|---|---|
| **Update Mechanism** | Traverses & diffs VDOM tree | Directly mutates bound DOM text cells & attributes + `$:` derived values |
| **Tailwind / UnoCSS Compilation** | Node.js + npm + Vite/PostCSS (`100–500 ms+`) | Native zero-dep Rust single pass (`~3–5 ms` total build) |
| **Memory Leak Prevention** | Garbage-collected tree diffing | Hierarchical scope disposal (`__clear_scope`) on `{#if}` / `{:else if}` / `{#each}` re-render |
| **Server Execution** | Separate backend stack + bundler | Built-in CPython stdio JSON-RPC supervisor (`POST /api/rpc`, supports `async def`) |
| **Compiler Binary** | Node.js / V8 startup overhead | Standalone `699.3 KB` Rust binary (`~2–5 ms` check/build) |

---

## 🔬 Reproducing These Benchmarks

```bash
# Build size-optimized release binary
cargo build --release

# Inspect binary size
ls -lh target/release/zvelte

# Time check & build on Tailwind + UnoCSS examples
./target/release/zvelte check examples/05-tailwind-dashboard
./target/release/zvelte build examples/05-tailwind-dashboard
./target/release/zvelte check examples/06-unocss-showcase
./target/release/zvelte build examples/06-unocss-showcase
```
