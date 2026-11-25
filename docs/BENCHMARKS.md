# ⚡ Zvelte Performance & Benchmarks

> Empirical benchmark measurements of the **Zvelte v0.0.7** Rust compiler + Tailwind CSS engine (`opt-level = "z"`, LTO, stripped) across the repository examples.

---

## 📊 Benchmark Summary (v0.0.7 — Rust Core + Built-In Tailwind Engine)

Measured on **Linux x86_64** using **Python 3.14.4** and **`target/release/zvelte` (`627,560` bytes / `612.9 KB`)**:

| Project | `zvelte check` (20-run mean) | `zvelte build` (20-run mean) | `dist/index.html` | `dist/app.js` |
|---|---|---|---|---|
| [`examples/01-counter`](../examples/01-counter/) | **4.12 ms** | **5.44 ms** | 1,918 B | 4,627 B |
| [`examples/02-todo-app`](../examples/02-todo-app/) | **3.73 ms** | **5.26 ms** | 2,265 B | 9,171 B |
| [`examples/03-python-api`](../examples/03-python-api/) | **3.32 ms** | **5.46 ms** | 1,568 B | 9,599 B |
| [`examples/04-ai-chatbot`](../examples/04-ai-chatbot/) | **4.87 ms** | **6.86 ms** | 3,289 B | 16,739 B |
| [`examples/05-tailwind-dashboard`](../examples/05-tailwind-dashboard/) *(98 Tailwind utilities + `@apply`)* | **2.13 ms** | **3.70 ms** | 7,298 B | 15,646 B |

> For historical Zig vs. Rust architecture benchmarks from the migration decision, see [`comparison.md`](../comparison.md).

---

## 🥊 Architecture Comparison: Fine-Grained Scoped DOM vs Virtual DOM

| Architectural Feature | Traditional Virtual DOM + PostCSS | Zvelte Rust + Tailwind Engine |
|---|---|---|
| **Update Mechanism** | Traverses & diffs VDOM tree | Directly mutates bound DOM text cells & attributes |
| **Tailwind Compilation** | Node.js + npm + `postcss` (`100–500 ms+`) | Native zero-dep Rust pass (`< 4 ms` total build) |
| **Memory Leak Prevention** | Garbage-collected tree diffing | Hierarchical scope disposal (`__clear_scope`) on `{#if}` / `{#each}` re-render |
| **Server Execution** | Separate backend stack + bundler | Built-in CPython stdio JSON-RPC supervisor (`POST /api/rpc`) |
| **Compiler Binary** | Node.js / V8 startup overhead | Standalone `612.9 KB` Rust binary (`2–7 ms` check/build) |

---

## 🔬 Reproducing These Benchmarks

```bash
# Build size-optimized release binary
cargo build --release

# Inspect binary size
ls -lh target/release/zvelte

# Time check & build on Tailwind + Python example
./target/release/zvelte check examples/05-tailwind-dashboard
./target/release/zvelte build examples/05-tailwind-dashboard
```
