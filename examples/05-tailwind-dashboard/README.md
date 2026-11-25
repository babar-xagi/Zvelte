# 🎨 Example 05: Full Tailwind CSS Dashboard + Python Backend

Demonstrates built-in **zero-dependency Tailwind CSS** support alongside **Python server RPC**:

- **Zero npm / node_modules:** Compiled directly by Zvelte's Rust Tailwind engine (`v0.0.7`).
- **Responsive Breakpoints & Dark Mode:** `md:grid-cols-3`, `md:col-span-2`, `dark:bg-slate-950`, `dark:text-slate-100`.
- **Opacity Modifiers, Rings, Gradients & Animations:** `bg-sky-500/10`, `focus:ring-2 focus:ring-sky-500/50`, `bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text`, `animate-pulse`.
- **Custom `@apply` Rules:** `.pill-badge { @apply inline-flex items-center ...; }` inside `<style>`.
- **Dynamic Class Toggling:** `<div class={dark_theme ? "dark" : ""}>`.

## 🚀 Running this Example

```bash
cd examples/05-tailwind-dashboard
zvelte dev
```

Visit [http://127.0.0.1:3005/](http://127.0.0.1:3005/) in your browser.
