# 📄 The `.zvelte` Component Format Specification

> Reference for authoring Python-first reactive single-file components in **Zvelte**.

---

## 📐 Component Anatomy

A `.zvelte` file unifies Python server code, reactive state, HTML template markup (including `<!-- ... -->` comments), and CSS in one file:

```html
<!-- app.zvelte -->
<script lang="python">
name = "Babar"
count = 0

def greeting(user):
    return f"Hello {user} from Python server!"
</script>

<main>
    <h1>Hello {name}</h1>
    <p>{greeting(name)}</p>
    <button click={count++}>Count: {count}</button>
</main>

<style>
main {
    font-family: system-ui, sans-serif;
    text-align: center;
}
</style>
```

---

## 🐍 1. Script Blocks & Execution Contexts

Attributes on `<script>` are optional and default sensibly:

| Script Tag | Language | Execution Context | Description |
|---|---|---|---|
| `<script lang="python">` | Python | Server | CPython worker process (`worker.py`). Ideal for APIs, AI models, math, and server state. |
| `<script>` | JavaScript | Client | Browser-side client script. |
| `<script context="server">` | Python | Server | Shorthand defaulting to Python server. |
| `<script lang="rust">` *(optional)* | Rust | Native (`cdylib`) | Optional native C-ABI module (`pub extern "C" fn`) compiled via `rustc` and linked into Python via `ctypes` (`mode = "hybrid"` or `"rust"`). |

### Safe Automatic State Extraction
Top-level literal assignments in `<script lang="python">` (e.g., `name = "Babar"`, `count = 0`, `flag = True`, `items = ["a", "b"]`) automatically become client reactive state.

Runtime server expressions (e.g., `GEMINI_KEY = os.getenv("GEMINI_API_KEY", "")` or `db = connect(...)`) are intentionally **skipped** from client state extraction so server secrets and Python APIs never leak into browser `app.js`.

### Python RPC Bridge (`POST /api/rpc`)
Top-level `def fn(...)` / `async def fn(...)` declarations in `<script lang="python">` automatically expose matching `async function fn(...args)` proxies in browser JavaScript, returning Promises resolved seamlessly by `{fn(...)}` text bindings and `.then(...)` event callbacks.

---

## ⚡ 2. Typed `<state>` Block

For explicit typing or array/dict declarations, you can also declare state in `<state>`:

```html
<state>
count: i32 = 0
is_active: bool = true
user_name: string = "Guest"
todos: []string = ["Learn Python", "Build with Zvelte"]
</state>
```

**Supported Types:** `i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `bool`, `string`, `any`, `list`, `dict`, `object`, `array`, and slice annotations like `[]string`, `[]i32`.

---

## 🌳 3. Template Markup, Directives & Comments

### HTML Comments
Standard HTML comments `<!-- ... -->` are skipped cleanly at root level, inside elements, and inside control blocks.

### Dynamic Text & Attributes (`{expression}`)
```html
<h1>{app_title}</h1>
<div class={active_class}>Count: {count}</div>
```

### Event Handlers (`event={expression}`)
Supported events: `click`, `input`, `change`, `submit`, `keydown`, `keyup`:
```html
<button click={count++}>Increment</button>
<button click={selected_model = "gemini"}>Gemini</button>
```

### Conditional Rendering (`{#if}` / `{:else}` / `{/if}`)
```html
{#if count > 5}
    <p class="highlight">High count!</p>
{:else}
    <p>Low count.</p>
{/if}
```

### List Rendering (`{#each}` / `{/each}`)
```html
{#each todos as todo}
    <div class="task">{todo}</div>
{/each}

<!-- With optional key syntax -->
{#each todos as todo (todo.id)}
    <div class="task">{todo.title}</div>
{/each}
```

### Two-Way Form Bindings (`bind:value` / `bind:checked`)
```html
<input type="text" bind:value={user_name} />
<input type="checkbox" bind:checked={is_done} />
```

---

## 🎨 4. Styles (`<style>`)

CSS declared inside `<style>...</style>` is injected into the generated HTML `<head>`.
