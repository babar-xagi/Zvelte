# 📄 The `.zvelte` Component Format Specification

> A comprehensive reference for authoring multi-language reactive single-file components in Zvelte.

---

## 📐 Anatomical Structure

Every `.zvelte` component is organized into clean, dedicated sections:

```html
<!-- 1. Multi-Language Script Blocks -->
<script lang="python" context="server"> ... </script>
<script lang="zig" context="native"> ... </script>
<script lang="zig" context="wasm"> ... </script>

<!-- 2. Reactive State Declaration Block -->
<state>
...
</state>

<!-- 3. Component Markup & UI Tree -->
<main>
...
</main>

<!-- 4. Scoped Styles -->
<style>
...
</style>
```

---

## 🐍 1. Script Blocks & Execution Contexts

All `<script>` tags must explicitly declare both `lang` and `context`. Zvelte does not guess or assume script semantics.

### Python Server Script (`lang="python" context="server"`)
Executes inside a managed CPython host worker supervisor. Ideal for database interactions, AI model pipelines (e.g. PyTorch, HuggingFace, llama-cpp-python), data science, and backend calculations.

```html
<script lang="python" context="server">
from datetime import datetime
import json

def ask_ai(prompt):
    # Any Python library, LLM call, or model pipeline can run here!
    return f"AI Response to '{prompt}': completed with high confidence."

def compute_factorial(n):
    import math
    return f"{n}! = {math.factorial(int(n))}"
</script>
```

#### How the RPC Bridge Works:
1. **Compilation / Supervisor:** Zvelte extracts this code and generates a line-buffered JSON-RPC runner (`worker.py`). The Zig dev server launches it as a supervised child process (`python3 -u`).
2. **Client Async Proxies:** The compiler generates browser JavaScript `async function ask_ai(...args)` proxies on `window`.
3. **HTTP Dispatch:** Calling `ask_ai("my prompt")` triggers a lightweight `POST /api/rpc` JSON request to the Zig server, which pipes it directly to Python's `stdin` and streams the JSON response back.
4. **Direct Reactive DOM Updates:** In event handlers and interpolations, results can be bound to reactive `<state>` variables or rendered directly into DOM text nodes via Promise resolution (`__setNodeText`).

```html
<main>
    <input type="text" bind:value={prompt} />
    <button click={ask_ai(prompt).then(res => { reply = res; update(); })}>
        Analyze with Python
    </button>
    <p>{reply}</p>
</main>
```

> [!NOTE]
> Server Python code is executed strictly on the server side in the CPython process. Proprietary logic, API keys, and model weights are never bundled or exposed in browser JavaScript assets.

---

### Zig Native Script (`lang="zig" context="native"`)
Compiled into a native C-ABI shared library (`libnative.so` / `libnative.dylib` / `native.dll`) for high-performance server-side compute acceleration.

```html
<script lang="zig" context="native">
pub export fn zvelte_multiply(a: i32, b: i32) callconv(.c) i32 {
    return a * b;
}

pub export fn zvelte_fast_fib(n: u32) callconv(.c) u64 {
    if (n <= 1) return n;
    var a: u64 = 0;
    var b: u64 = 1;
    var i: u32 = 2;
    while (i <= n) : (i += 1) {
        const next = a + b;
        a = b;
        b = next;
    }
    return b;
}
</script>
```

#### How the Native Compilation & Python `ctypes` Linker Works:
1. **Ahead-of-Time Native Compilation:** Zvelte extracts this code to `native.zig` and invokes `zig build-lib native.zig -dynamic -O ReleaseFast` to generate an optimized shared library (`libnative.so`).
2. **Automated Python `ctypes` Wrapper:** Zvelte generates `worker.py` with automatic `ctypes.CDLL` loading, binding `argtypes` and `restype` for all exported C-ABI functions (`export fn ... callconv(.c)`).
3. **Zero-Overhead Python Ingestion:** Exported Zig functions are injected into Python's global namespace, so Python server code can invoke native Zig functions at bare-metal machine code speed:
```python
def benchmark_native_fib(n):
    # Calls compiled Zig shared library via ctypes with zero overhead!
    val = zvelte_fast_fib(int(n))
    return f"Zig Native Fibonacci({n}) = {val}"
```
4. **Client Asynchronous RPC Proxies:** Zvelte also generates client-side `async function <fn_name>(...args)` proxies in JavaScript, so browser UI code can invoke native Zig functions directly via JSON-RPC or through Python wrappers:
```javascript
const result = await zvelte_fast_fib(40); // 102334155
```
```

---

### Zig WebAssembly Script (`lang="zig" context="wasm"`)
Compiled targeting `wasm32-freestanding` and loaded by the browser runtime for client-side compute acceleration.

```html
<script lang="zig" context="wasm">
pub export fn zvelte_hash(input: u32) callconv(.c) u32 {
    return (input ^ 0x5bf03635) *% 0x1d8e4e27;
}
</script>
```

---

## ⚡ 2. The `<state>` Declaration Block

The `<state>` block is a typed, language-neutral DSL for declaring client-side reactive state cells.

### Syntax
```text
<variable_name>: <type> = <initial_value>
```

### Supported Types

| Type | Description | Example |
|---|---|---|
| `i32` | 32-bit signed integer | `count: i32 = 0` |
| `i64` | 64-bit signed integer | `timestamp: i64 = 1700000000` |
| `u32` | 32-bit unsigned integer | `items_left: u32 = 5` |
| `u64` | 64-bit unsigned integer | `visitor_count: u64 = 100000` |
| `f32` | 32-bit floating point | `rating: f32 = 4.8` |
| `f64` | 64-bit floating point | `price: f64 = 29.99` |
| `bool` | Boolean flag | `is_open: bool = false` |
| `string` | UTF-8 string literal | `title: string = "Zvelte App"` |

### Example
```html
<state>
count: i32 = 0
is_active: bool = true
app_title: string = "Dashboard"
</state>
```

---

## 🌳 3. Template Markup & Interpolation

The markup syntax looks like standard HTML with dynamic extensions:

### Dynamic Expressions (`{expression}`)
Interpolate state variables or functions directly into text or element attributes:

```html
<h1>{app_title}</h1>
<p>Current counter: {count}</p>
```

### Event Bindings (`event={expression}`)
Attach interactive event listeners directly:

```html
<!-- Increment state -->
<button click={count++}>Increment</button>

<!-- Decrement state -->
<button click={count--}>Decrement</button>

<!-- Set state directly -->
<button click={count = 0}>Reset</button>
```

### Conditional Rendering (`{#if}` / `{:else}`)
Render elements conditionally based on state expressions:

```html
{#if count > 5}
    <p class="highlight">Count is greater than 5!</p>
{:else}
    <p>Count is low.</p>
{/if}
```

### Keyed List Rendering (`{#each}` blocks)
Iterate over array/slice state variables dynamically:

```html
{#each items as item}
    <div class="list-item">{item}</div>
{/each}

<!-- With key expression -->
{#each todos as todo (todo.id)}
    <div class="todo-item">{todo.title}</div>
{/each}
```

### Two-Way Input Binding (`bind:value` & `bind:checked`)
Synchronize form input elements directly with reactive state cells:

```html
<!-- Text input binding -->
<input type="text" bind:value={user_name} />

<!-- Checkbox boolean binding -->
<input type="checkbox" bind:checked={is_completed} />
```

---

## 🎨 4. Scoped Styles (`<style>`)

CSS styles declared in `<style>` apply directly to the component template:

```html
<style>
main {
    max-width: 48rem;
    margin: 2rem auto;
    font-family: system-ui, -apple-system, sans-serif;
    color: #1a1a1a;
}

button {
    background-color: #f05a28;
    color: white;
    border: none;
    padding: 0.6rem 1.2rem;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease-in-out;
}

button:hover {
    background-color: #d04516;
}
</style>
```
