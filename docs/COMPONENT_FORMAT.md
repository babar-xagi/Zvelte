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

## 🎨 4. Styles, Tailwind CSS & UnoCSS (`<style>`, Attributify & Utility Classes)

CSS declared inside `<style>...</style>` is injected into the generated HTML `<head>`. In addition, **Zvelte (`v0.0.8+`) includes built-in zero-dependency Tailwind CSS (`v0.0.7+`) and UnoCSS (`v0.0.8+`) engines**:

- **Utility Classes Directly in HTML:** Write `class="flex items-center justify-between p-6 bg-slate-900 text-sky-400 rounded-xl shadow-lg"` with zero `npm` or `node_modules`.
- **Full Variants & Breakpoints:** Stack modifiers freely — `dark:md:hover:bg-indigo-600/90`, `focus:ring-2`, `group-hover:opacity-100`, `peer-checked:bg-emerald-500`, `before:content-['*']`, `!p-4` / `p-6!`, plus UnoCSS `lt-sm`..`lt-2xl`, `hocus:`, `children:`, `all:`, and `not-*`.
- **UnoCSS Attributify Mode:** Write boolean utilities (`<header flex items-center pb-6 bg-slate-900/80>`) and grouped attribute selectors (`bg="slate-950/60 hover:slate-900"`, `p="x-6 y-8"`, `border="~ slate-800 rounded-xl"`, `hover="bg-sky-500 text-white"`). Standard HTML (`type`, `size` on inputs, `placeholder`, `data-*`, `aria-*`, SVG presentation attrs) are preserved automatically.
- **UnoCSS Variant Groups (`variant:(a b)`):** Nested variant groups like `dark:(bg-slate-900/90 hover:(border-sky-500/60 scale-105))` are expanded in both CSS rules AND runtime JS DOM bindings (`setAttribute`), while arbitrary CSS parentheses (`bg-[rgb(1,2,3)]`, `w-[calc(100%-2rem)]`) stay intact.
- **UnoCSS Preset Shorthands & `un-` Prefix:** `flex-center`, `flex-between`, `flex-col-center`, `grid-center`, `c-*`, `op-*`, `fw-*`, `rd*`, `pa-*`, `b*`, `pos-*`, `of-*`, `ma`/`mxa`/`mya`, and optional `un-flex` / `un-text-emerald-400`.
- **Shortcuts (`[unocss.shortcuts]`):** Reusable shortcuts defined in `app.toml` expand cleanly before utility layers so explicit classes always override.
- **Arbitrary Values & Colors:** `w-[320px]`, `bg-[#1e293b]/80`, `z-[100]`, `[mask-type:alpha]`.
- **Dynamic Reactive Classes:** Classes inside ternary expressions (`class={open ? "hover:(bg-emerald-500 scale-105)" : "hidden"}`) and backtick templates (`` class={`p-4 ${dark ? 'bg-slate-950' : 'bg-white'}`} ``) are automatically extracted and compiled.
- **CSS `@apply`, `--at-apply:`, `@screen` & `theme()` inside `<style>`:**
  ```html
  <style>
  @unocss preflights;
  :root { --accent: theme('colors.sky.400'); }
  .pill-badge {
      --at-apply: inline-flex items-center px-3 py-1 rd-full text-xs fw-semibold bg-sky-500/10 c-sky-400;
  }
  @screen md {
      .hero-grid { grid-template-columns: 1.4fr 1fr; }
  }
  </style>
  ```
- **Zero Overhead When Unused:** If a component uses only custom CSS classes and does not explicitly configure `[tailwind]` or `[unocss]` in `app.toml`, no Preflight or extra utility CSS is injected.
