# Python Server Integration Example (Zvelte)

This example demonstrates how **Zvelte** empowers Python developers to build reactive web apps without writing backend boilerplate:

1. **`<script lang="python">`**: Server-side CPython worker executing functions over line-buffered JSON-RPC.
2. **Fine-Grained DOM Updates**: Client state updates and server function calls without Virtual DOM overhead.
3. **Rust-Powered Speed**: The compiler, bundler, and dev server are written in Rust for sub-millisecond execution.

## Running the App

```bash
zvelte check examples/03-python-api
zvelte build examples/03-python-api
zvelte dev examples/03-python-api
```
