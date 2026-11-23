#!/usr/bin/env bash
set -euo pipefail

echo "========================================================"
echo "🚀 Publishing GitHub Releases for Zvelte (v0.0.2 & v0.0.3)"
echo "========================================================"

if ! gh auth status >/dev/null 2>&1; then
    echo "⚠️ GitHub CLI is not authenticated."
    echo "Please run: gh auth login"
    echo "Then re-run this script: ./scripts/publish_releases.sh"
    exit 1
fi

echo "📦 Creating Release v0.0.2..."
gh release create v0.0.2 \
    --title "v0.0.2: Template Language Essentials & Advanced Reactivity" \
    --notes "### ⚡ Template Language Essentials & Advanced Reactivity (Phase 8)

#### ✨ Core Features
- **Conditional Rendering Blocks (\`{#if}\` / \`{:else}\` / \`{/if}\`):** Supports conditional template rendering with \`display: contents\` wrapper containers and fine-grained subscriber recreation without Virtual DOM diffing.
- **Keyed List Rendering Blocks (\`{#each items as item}\` / \`{#each items as item (key)}\`):** Dynamic loop iteration over reactive array collections in state, generating individual element templates with lexical scope preservation.
- **Two-Way Form Input Bindings (\`bind:value\` / \`bind:checked\`):** Direct two-way synchronization on \`<input>\` elements with cursor stabilization checks preventing input jumping.
- **Advanced Expression Parser:** Automatic lexical scoping for complex expressions, mathematical operations, array mutations (\`todos.push(...)\`), and ternary conditions.
- **Extended State Types:** Added support for array/slice types (\`[]string\`, \`[]i32\`, \`list\`, \`array\`) in \`<state>\` blocks.
- **Enhanced Example Application:** Updated \`examples/02-todo-app\` into a fully interactive task manager.

#### 🧪 Verification
- All 18 unit tests passing (\`zig build test\`).
- Verified on \`examples/01-counter\` and \`examples/02-todo-app\`."

echo "✔ Release v0.0.2 published successfully."

echo "📦 Creating Release v0.0.3..."
gh release create v0.0.3 \
    --title "v0.0.3: Python Server Worker Process Integration & JSON-RPC Bridge" \
    --notes "### ⚡ Python Server Worker Process Integration & JSON-RPC Bridge (Phase 10)

#### ✨ Core Features
- **Embedded CPython Worker Supervisor:** The Zig dev server automatically detects \`<script lang=\"python\" context=\"server\">\`, generates an isolated \`.zvelte/dev/worker.py\` runner, and manages a live \`python3 -u\` child process with line-buffered pipe I/O.
- **Line-Buffered JSON-RPC IPC Bridge:** Implemented high-performance IPC over standard I/O streams (\`stdin\`/\`stdout\`). Eliminates port collision and achieves sub-millisecond local request latency.
- **HTTP RPC API Gateway (\`POST /api/rpc\`):** The embedded Zig HTTP server dispatches RPC calls to the Python worker and streams JSON responses back with CORS preflight support.
- **Client Asynchronous RPC Proxies:** JavaScript code generator automatically detects all declared top-level Python functions (\`def ...\` and \`async def ...\`) and emits \`async function <fn_name>(...args)\` wrappers.
- **Promise-Aware Reactive DOM Resolution:** Generated runtime includes \`__setNodeText(node, val)\` supporting Promise-based dynamic interpolations and event handlers.
- **Semantic Analyzer Function Resolution:** Enhanced symbol analyzer to extract functions declared in \`<script>\` blocks and permit their invocation in event handlers.
- **Graceful Process Management:** Automatic child process shutdown with SIGTERM termination and pipe deinitialization on server exit.
- **Production Worker Output:** \`zvelte build\` emits standalone \`dist/worker.py\` alongside \`dist/index.html\` and \`dist/app.js\` with asset size tracking.

#### 🧪 Verification
- All 23 unit tests passing (\`zig build test\`).
- Verified across \`examples/01-counter\`, \`examples/02-todo-app\`, and \`examples/03-hybrid-python-zig\`."

echo "✔ Release v0.0.3 published successfully."
echo "🎉 All GitHub releases are live at: https://github.com/babar-xagi/Zvelte/releases"
