# Hybrid Python + Rust Example (Rustby / Zvelte-RS)

This project demonstrates the **Rustby** implementation of the Zvelte framework architecture, powered completely by Rust and supporting:
1. **`<script lang="rust">`**: Ahead-of-time compiled Rust functions via `rustc --crate-type cdylib -O`, exposing clean C-ABI entry points.
2. **`<script lang="python">`**: Server-side CPython worker executing asynchronous JSON-RPC calls and communicating with native Rust via `ctypes`.
3. **Reactive HTML / Svelte Syntax**: Pure client reactivity with automatic state extraction (`{count}`, `{user_name}`), inline event expressions (`click={count++}`), and dynamic interpolation.

## Running with Rustby

```bash
# From workspace root:
./rustby/target/release/zvelte-rs check examples/05-hybrid-python-rust
./rustby/target/release/zvelte-rs build examples/05-hybrid-python-rust
./rustby/target/release/zvelte-rs dev examples/05-hybrid-python-rust
```
