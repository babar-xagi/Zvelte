# Zvelte

> **A high-performance, multi-language reactive web framework powered by Rust, supporting Python server blocks, native Rust C-ABI routines, and fine-grained DOM reactivity.**

[![Rust 1.80+](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org/)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10+-blue.svg)](https://www.python.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Release: v0.0.5](https://img.shields.io/badge/Release-v0.0.5-brightgreen.svg)](https://github.com/babar-xagi/Zvelte/releases/tag/v0.0.5)

---

## 🌟 Highlights

- ⚡ **Deterministic Two-File Scaffold:** `zvelte new my-app` generates **strictly** `app.toml` and `app.zvelte`. Zero configuration noise, no `package.json`, no webpack, no node_modules.
- 🎯 **Fine-Grained Reactivity:** Zero Virtual DOM overhead. Reactive state bindings directly update affected DOM text cells and attributes, identical to Svelte.
- 🐍 **Python Server Integration:** Seamless CPython worker executing line-buffered JSON-RPC over stdin/stdout. Safely access databases, file systems, and AI models on the server.
- 🦀 **Native Rust Acceleration:** Write `<script lang="rust">` blocks that compile directly via `rustc --crate-type cdylib -O` to native machine code (`libnative.so` / `native.dll`), linked to Python via `ctypes`.
- 🚀 **Zero-Dependency Rust Engine:** The entire compiler, parser, codegen, and dev server are written in pure Rust using only the standard library (`std`), delivering sub-500 KB binaries and sub-3ms check times.

---

## 🤖 Live Showcase: AI Chatbot in a Single Component

Build high-performance, AI-native web apps with Python and Google Gemini / OpenAI inside a single `.zvelte` component—with zero npm, zero pip dependencies, and zero API key leakage:

```html
<!-- app.zvelte -->
<script lang="python">
import os, json, urllib.request

GEMINI_KEY = os.getenv("GEMINI_API_KEY", "")

def ask_ai(prompt):
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={GEMINI_KEY}"
    req = urllib.request.Request(
        url,
        data=json.dumps({"contents":[{"parts":[{"text": prompt}]}]}).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=15) as resp:
        return json.loads(resp.read().decode("utf-8"))["candidates"][0]["content"]["parts"][0]["text"]
</script>

<state>
user_prompt: string = ""
ai_reply: string = ""
is_loading: bool = false
</state>

<main>
    <input type="text" bind:value={user_prompt} placeholder="Ask Gemini anything..." />
    <button click={
        is_loading = true;
        const p = user_prompt;
        user_prompt = "";
        ask_ai(p).then(res => { ai_reply = res; is_loading = false; update(); });
    }>Send Prompt</button>

    {#if is_loading}
        <p>Thinking & streaming from Python server...</p>
    {/if}
    {#if ai_reply}
        <div class="bubble">{ai_reply}</div>
    {/if}
</main>
```

> 🛡️ **Zero API Key Leakage:** `GEMINI_API_KEY` stays strictly in the isolated CPython worker process. It is never bundled or exposed in browser JavaScript assets. Run it with `cd examples/04-ai-chatbot && zvelte dev`!

---

## 💡 Code Examples

Browse working example applications in the [`examples/`](examples/) directory:

- ⚡ **[01-counter](examples/01-counter/):** Pure reactive client counter with increment, decrement, and reset.
- 📝 **[02-todo-app](examples/02-todo-app/):** Task manager illustrating multiple state variables, two-way bindings, and loops.
- 🦀 **[03-hybrid-python-rust](examples/03-hybrid-python-rust/):** Multi-language hybrid app with Python server logic and native Rust C-ABI functions.
- 🤖 **[04-ai-chatbot](examples/04-ai-chatbot/):** Full AI chatbot supporting Google Gemini & OpenAI GPT-4o with server-side API keys and zero pip dependencies.

---

## 🚀 Quick Start

### Installation

```bash
# 1. Clone the repository
git clone git@github.com:babar-xagi/Zvelte.git
cd Zvelte

# 2. Build optimized release binary via Cargo
cargo build --release

# 3. Copy or link to your PATH
cp target/release/zvelte ~/.local/bin/

# 4. Verify
zvelte --version
```

### Create Your First App

```bash
# 1. Create a project (strictly produces app.toml + app.zvelte)
zvelte new my-app
cd my-app

# 2. Start the local development server
zvelte dev
# Open http://127.0.0.1:3000/

# 3. Compile for production
zvelte build
# Emits optimized dist/index.html, dist/app.js, dist/worker.py, and dist/libnative.so
```

---

## 📐 Component Anatomy (`app.zvelte`)

Declare variables and functions inside `<script>`, and bind them directly in your markup with Svelte-style ergonomics:

```html
<script lang="rust">
#[no_mangle]
pub extern "C" fn rust_fast_fib(n: u32) -> u64 {
    if n <= 1 { return n as u64; }
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}
</script>

<script lang="python">
name = "Babar"
count = 0

def greeting(user):
    return f"Hello {user}! Greetings from Python."

def benchmark_rust_fib(n):
    val = rust_fast_fib(int(n))
    return f"Rust Native Fibonacci({n}) = {val:,}"
</script>

<main>
    <h1>Hello {name}</h1>
    <p>{greeting(name)}</p>

    <div class="card">
        <button click={count++}>
            Clicked {count} times
        </button>
        <button click={count = 0}>
            Reset
        </button>
    </div>

    <div class="card">
        <p>{benchmark_rust_fib(40)}</p>
    </div>
</main>

<style>
main {
    font-family: system-ui, sans-serif;
    max-width: 36rem;
    margin: 3rem auto;
    text-align: center;
}
button {
    margin: 0.5rem;
    padding: 0.6rem 1.2rem;
    border-radius: 6px;
    background: #0070f3;
    color: white;
    cursor: pointer;
}
</style>
```

---

## 🏗️ Architecture Diagram

```mermaid
flowchart TD
    subgraph Inputs["Source Files (Deterministic 2-File Layout)"]
        TOML["app.toml"]
        SFC["app.zvelte"]
    end

    subgraph Compiler["Zvelte Compiler Engine (Rust Core)"]
        CFG["Config Parser (toml_parser.rs)"]
        PARSER["Multi-Lang SFC Parser (parser.rs)"]
        SEM["Semantic Symbol Analyzer (analyzer.rs)"]
        NATIVE["Native Rust C-ABI Compiler (rustc cdylib)"]
        PYGEN["Python ctypes & RPC Supervisor Gen (native_compiler.rs)"]
        JSGEN["Fine-Grained DOM & Async Proxy Codegen (js_codegen.rs)"]
    end

    subgraph Runtime["Execution Backends"]
        NATIVELIB["libnative.so / native.dll (Rust C-ABI)"]
        PYWORKER["worker.py (CPython + ctypes Linker)"]
        CLIENT["Browser Client (Fine-Grained DOM + RPC Bridge)"]
    end

    TOML --> CFG
    SFC --> PARSER
    PARSER --> SEM
    SEM --> NATIVE
    SEM --> PYGEN
    SEM --> JSGEN

    NATIVE --> NATIVELIB
    PYGEN --> PYWORKER
    NATIVELIB -. ctypes .-> PYWORKER
    JSGEN --> CLIENT
    CLIENT == JSON-RPC (HTTP /api/rpc) ==> PYWORKER
```

---

## 🧪 Testing

Run the full suite of unit and integration tests:

```bash
cargo test
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
