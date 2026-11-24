# Hybrid Python + Rust Example (Zvelte)

This project demonstrates the **Zvelte** multi-language architecture, powered by the Rust compiler core:
1. **`<script lang="rust">`**: Ahead-of-time compiled Rust functions via `rustc --crate-type cdylib -O`, exposing clean C-ABI entry points.
2. **`<script lang="python">`**: Server-side CPython worker executing asynchronous JSON-RPC calls and communicating with native Rust via `ctypes`.
3. **Reactive HTML / Svelte Syntax**: Pure client reactivity with automatic state extraction (`{count}`, `{user_name}`), inline event expressions (`click={count++}`), and dynamic interpolation.

## Running the App

```bash
# Validate component and scripts
zvelte check examples/03-hybrid-python-rust

# Build production bundle
zvelte build examples/03-hybrid-python-rust

# Start development server
zvelte dev examples/03-hybrid-python-rust
```
