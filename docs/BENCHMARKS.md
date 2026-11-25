# ⚡ Zvelte Performance & Benchmarks

> Empirical benchmark measurements of the **Zvelte v0.0.8** Rust compiler + Tailwind CSS & UnoCSS engines (`opt-level = "z"`, LTO, stripped) across the repository examples.

---

## 📊 Benchmark Summary (v0.0.8 — Rust Core + Tailwind & UnoCSS Engines)

Measured on **Linux x86_64** using **Python 3.14.4** and **`target/release/zvelte` (`687,856` bytes / `671.7 KB`)**:

| Project | `zvelte check` (20-run mean) | `zvelte build` (20-run mean) | `dist/index.html` | `dist/app.js` |
|---|---|---|---|---|
| [`examples/01-counter`](../examples/01-counter/) | **2.14 ms** | **2.77 ms** | 1,918 B | 4,627 B |
| [`examples/02-todo-app`](../examples/02-todo-app/) | **1.99 ms** | **2.96 ms** | 2,265 B | 9,171 B |
| [`examples/03-python-api`](../examples/03-python-api/) | **2.03 ms** | **3.09 ms** | 1,568 B | 9,599 B |
| [`examples/04-ai-chatbot`](../examples/04-ai-chatbot/) | **2.98 ms** | **4.16 ms** | 3,289 B | 16,739 B |
| [`examples/05-tailwind-dashboard`](../examples/05-tailwind-dashboard/) *(98 Tailwind utilities + `@apply`)* | **1.98 ms** | **3.40 ms** | 7,298 B | 15,646 B |
| [`examples/06-unocss-showcase`](../examples/06-unocss-showcase/) *(95 UnoCSS rules + 13 utilities + Attributify + Shortcuts)* | **2.09 ms** | **4.11 ms** | 7,465 B | 19,730 B |

> For historical Zig vs. Rust architecture benchmarks from the migration decision, see [`comparison.md`](../comparison.md).

---

## 🥊 Architecture Comparison: Fine-Grained Scoped DOM vs Virtual DOM

| Architectural Feature | Traditional Virtual DOM + PostCSS / Node UnoCSS | Zvelte Rust + Tailwind / UnoCSS Engine |
|---|---|---|
| **Update Mechanism** | Traverses & diffs VDOM tree | Directly mutates bound DOM text cells & attributes |
| **Tailwind / UnoCSS Compilation** | Node.js + npm + Vite/PostCSS (`100–500 ms+`) | Native zero-dep Rust single pass (`~3–4 ms` total build) |
| **Memory Leak Prevention** | Garbage-collected tree diffing | Hierarchical scope disposal (`__clear_scope`) on `{#if}` / `{#each}` re-render |
| **Server Execution** | Separate backend stack + bundler | Built-in CPython stdio JSON-RPC supervisor (`POST /api/rpc`) |
| **Compiler Binary** | Node.js / V8 startup overhead | Standalone `671.7 KB` Rust binary (`~2–4 ms` check/build) |

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
