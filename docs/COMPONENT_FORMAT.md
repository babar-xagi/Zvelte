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
Executes inside a CPython host worker. Ideal for database interactions, AI model pipelines, and backend routing.

```html
<script lang="python" context="server">
from datetime import datetime

user = "Developer"

def get_welcome_message():
    current_time = datetime.now().strftime("%H:%M")
    return f"Welcome {user}! Current time: {current_time}"
</script>
```

> [!NOTE]
> Server Python code is executed strictly on the server side. It is never bundled or leaked into the browser JavaScript assets.

---

### Zig Native Script (`lang="zig" context="native"`)
Compiled into a native C-ABI shared library for server-side compute acceleration.

```html
<script lang="zig" context="native">
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
