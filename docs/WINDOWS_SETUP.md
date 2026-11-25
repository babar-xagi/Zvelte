# 🪟 Windows Setup and User Guide for Zvelte

> Complete step-by-step instructions for installing, building, and using **Zvelte** (`v0.0.7`, Rust Core + Python + Tailwind CSS) on **Windows 10 / 11** using PowerShell or Windows Terminal.

---

## 📋 System Requirements

- **Operating System:** Windows 10 (Build 19041+) or Windows 11 (64-bit)
- **Rust Toolchain:** Rust 1.85+ (`cargo` + `rustc`, supporting Rust 2024 edition)
- **Python:** Python `3.10+` (`python.exe` on `PATH`)
- **Terminal:** Windows Terminal / PowerShell 7+ (recommended)

---

## 🛠️ Step 1: Install Prerequisites

### 1. Install Rust (`rustup`)
Install Rust via `winget` or [rustup.rs](https://rustup.rs/):
```powershell
winget install -e --id Rustlang.Rustup
```
Verify in a fresh PowerShell window:
```powershell
rustc --version
cargo --version
```

### 2. Install Python
```powershell
winget install -e --id Python.Python.3.13
```
*(If using the installer from [python.org](https://www.python.org/downloads/windows/), ensure **"Add python.exe to PATH"** is checked.)*

Verify:
```powershell
python --version
```

---

## 🚀 Step 2: Build & Install Zvelte on Windows

```powershell
# 1. Clone the repository
git clone https://github.com/babar-xagi/Zvelte.git
cd Zvelte

# 2. Run the test suite & build optimized release binary
cargo test
cargo build --release

# 3. Install zvelte.exe to a local bin folder
$binPath = "$HOME\.local\bin"
if (!(Test-Path $binPath)) { New-Item -ItemType Directory -Path $binPath }
Copy-Item ".\target\release\zvelte.exe" "$binPath\zvelte.exe"

# 4. Add to User PATH if needed
if ($env:Path -notlike "*$binPath*") {
    [Environment]::SetEnvironmentVariable("Path", $env:Path + ";$binPath", "User")
    $env:Path += ";$binPath"
}

# 5. Verify installation
zvelte --version
```

---

## 🎯 Step 3: Create & Run Your First App

```powershell
zvelte new myapp
cd myapp
zvelte dev
```

You will see:
```text
  ✔ Python server worker supervisor online

  ✔ Zvelte dev server running at http://127.0.0.1:3000/
```

Open [http://127.0.0.1:3000/](http://127.0.0.1:3000/) in Edge or Chrome. Production builds (`zvelte build`) emit standalone assets inside `dist/`.

---

## 🔧 Windows Compatibility & Troubleshooting

- **Cross-Platform Networking (`TcpListener`):** Zvelte uses Rust's standard `std::net::TcpListener`, avoiding Linux-only syscalls (`BindFailed`). If port `3000` is in use, `zvelte dev` automatically tries `3001..3009`.
- **Windows CRLF Line Endings (`\r\n`):** Zvelte properly escapes `\r` and `\n` in generated JavaScript strings, so `.zvelte` files saved with Windows CRLF line endings compile cleanly without JS syntax errors.
- **Python Executable Auto-Detection:** Zvelte tries `[python] executable`, then `python3`, then `python` so Windows `python.exe` installations are detected automatically.
- **UTF-8 Symbols in PowerShell:** If box/checkmark characters render strangely in legacy Windows PowerShell, run `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8` or use **Windows Terminal**.
