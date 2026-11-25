# 🚀 Getting Started with Zvelte

> Step-by-step guide to building and installing the Rust-powered **Zvelte** compiler and creating your first **Python + Svelte** reactive web application.

---

## 📋 Prerequisites

1. **Rust Toolchain (`cargo` & `rustc`):** Needed to compile the `zvelte` CLI binary from source (`rustc 1.85+` / Rust 2024 edition).
   ```bash
   rustc --version
   cargo --version
   ```
2. **Python 3.10+:** Powers `<script lang="python">` server workers via CPython JSON-RPC.
   ```bash
   python3 --version
   ```
3. **Supported Operating Systems:**
   - Linux (Ubuntu 22.04+, Debian, Fedora, Arch) & WSL2
   - Windows 10 / 11 (see [Windows Setup Guide](WINDOWS_SETUP.md))
   - macOS (Apple Silicon & Intel)

---

## 🛠️ Step 1: Build & Install Zvelte

Clone the repository and compile the zero-dependency Rust release binary (`~672 KB`, includes built-in Tailwind CSS & UnoCSS compilers):

```bash
# 1. Clone the repository
git clone git@github.com:babar-xagi/Zvelte.git
cd Zvelte

# 2. Run unit & integration tests
cargo test

# 3. Build optimized release binary
cargo build --release

# 4. Install to your local user bin directory
mkdir -p ~/.local/bin
cp target/release/zvelte ~/.local/bin/

# 5. Verify installation
zvelte --version
```

Expected output:
```text
zvelte version 0.0.8
```

---

## 📦 Step 2: Create Your First Project

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

Your project directory contains **strictly two files**:
- `app.toml`: Project settings (`mode = "python"`, port, hot reload, build output).
- `app.zvelte`: Single-file component with `<script lang="python">`, reactive markup, and CSS.

---

## 💻 Step 3: Start the Live Development Server

```bash
zvelte dev
```

Output:
```text
  ✔ Python server worker supervisor online

  ✔ Zvelte dev server running at http://127.0.0.1:3000/
```

Open [http://127.0.0.1:3000/](http://127.0.0.1:3000/) in your browser. Edit `app.zvelte` or `app.toml` and save — Zvelte automatically recompiles, restarts the Python worker, and refreshes your browser tab!

---

## 🧪 Step 4: Validate & Build for Production

```bash
# Validate syntax and state references
zvelte check

# Compile production assets to dist/
zvelte build

# Clean generated dist/ and .zvelte/ artifacts cleanly
zvelte clean
```
