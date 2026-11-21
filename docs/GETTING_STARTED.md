# 🚀 Getting Started with Zvelte

> Step-by-step guide to installing the Zvelte toolchain, scaffolding your first project, running the local development server, and building for production.

---

## 📋 Prerequisites

Before getting started, make sure your system has:

1. **Zig 0.17.1:**
   ```bash
   zig version
   # Expected output: 0.17.1
   ```
2. **Python 3.14+ (or 3.15):**
   ```bash
   python3 --version
   ```
3. **Supported Operating Systems:**
   - Linux (Ubuntu 22.04+, Debian, Fedora, Arch)
   - WSL2 (Windows Subsystem for Linux)
   - macOS (Apple Silicon & Intel)

---

## 🛠️ Step 1: Building & Installing the Zvelte Compiler

Clone the Zvelte repository and compile the native executable:

```bash
# 1. Clone the repository
git clone git@github.com:babar-xagi/Zvelte.git
cd Zvelte

# 2. Build in release mode
zig build -Doptimize=ReleaseFast

# 3. Install to your local user binary path
cp zig-out/bin/zvelte ~/.local/bin/

# 4. Verify installation
zvelte --version
```

You should see:
```text
zvelte version 0.0.1
```

---

## 📦 Step 2: Creating Your First Project

Run `zvelte new` followed by your desired project name:

```bash
zvelte new my-app
cd my-app
```

Output:
```text
✔ Created Zvelte project my-app
  ├── app.toml
  └── app.zvelte

Next steps:
  cd my-app
  zvelte dev
```

### 🔍 Inspecting the Two Files

Check your project directory:

```bash
ls -la
```

Notice that there are **strictly two files**:
- `app.toml`: All project configuration, server ports, and compiler settings.
- `app.zvelte`: Your multi-language component containing scripts, reactive state, markup, and styles.

No `package.json`, no `build.zig`, no `node_modules/`, and no cluttered boilerplate.

---

## 💻 Step 3: Running the Development Server

Start the zero-config development server:

```bash
zvelte dev
```

Output:
```text
✔ Zvelte dev server running at http://127.0.0.1:3000/
```

Open [http://127.0.0.1:3000/](http://127.0.0.1:3000/) in your web browser. You will see:
- Greeting text rendered dynamically from the component.
- An interactive counter displaying `Count: 0`.
- **Increment** and **Reset** buttons updating the counter live with zero page reloads.

---

## 🧪 Step 4: Semantic Checking

To inspect your code for type errors, missing state variables, or incompatible script language modes without starting the server, run:

```bash
zvelte check
```

Output:
```text
✔ Check passed: 'my-app' (mode: hybrid, renderer: js, 1 state vars, 3 scripts)
```

---

## 🏗️ Step 5: Building for Production

When you are ready to deploy your application to any static host, CDN, or web server:

```bash
zvelte build
```

Output:
```text
Building Zvelte project for production...
✔ Production build complete in dist/
  ├── index.html (373 bytes)
  └── app.js (1545 bytes)
```

The production assets in `dist/` are fully optimized and ready to serve on GitHub Pages, Vercel, Netlify, Cloudflare Pages, Nginx, or Caddy.

---

## 🧹 Step 6: Cleaning Build Artifacts

To remove all generated caches (`.zvelte/`) and production build directories (`dist/`), run:

```bash
zvelte clean
```

Output:
```text
Cleaning Zvelte build artifacts and cache...
✔ Clean complete. User source files preserved.
```

Your project directory is returned to its pristine two-file state!
