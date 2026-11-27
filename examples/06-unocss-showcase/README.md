# ⚡ Example 06: Atomic UnoCSS Showcase

Interactive UnoCSS + Python RPC showcase built into Zvelte v0.0.8 with **zero Node.js dependencies**.

## ✨ Features Demonstrated

- **Attributify Mode:** Boolean utility attributes (`<header flex items-center justify-between pb-6>`) and grouped attribute selectors (`bg="slate-950/60 hover:slate-900"`, `p="x-6 y-8"`, `border="~ slate-800 rounded-xl"`).
- **Nested Variant Groups:** Automatically expanded in both compiled CSS and runtime DOM bindings (`dark:(bg-slate-900/90 hover:(border-sky-500/60))`, `hover:(bg-emerald-400 scale-105)`).
- **TOML Shortcuts:** Composable shortcuts (`pill`, `uno-card`, `uno-btn`, `uno-btn-primary`) configured in `[unocss.shortcuts]` inside `app.toml`.
- **Preset-Uno Shorthands & Variants:** `c-*`, `op-*`, `fw-*`, `rd-*`, `pa-*`, `mxa`, `flex-between`, `pos-relative`, `lt-md:*`, `hocus:*`, trailing `!` important syntax (`p-6!`), and optional `un-` prefixing.
- **UnoCSS Directives in `<style>`:** `@unocss preflights;`, `--at-apply:`, `@screen md`, and `theme('colors.sky.400')`.
- **Python Backend RPC:** Live preset inspection via `analyze_tokens(preset_name, token_count)`.

## 🚀 Run

```bash
zvelte check examples/06-unocss-showcase
zvelte build examples/06-unocss-showcase
zvelte dev examples/06-unocss-showcase
```
