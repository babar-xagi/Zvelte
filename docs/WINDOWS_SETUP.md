# 🪟 Windows Setup and User Guide for Zvelte

> Complete step-by-step instructions for installing, configuring, and building with **Zvelte** on **Windows 10 / 11** using PowerShell or Windows Terminal.

---

## 📋 System Requirements

- **Operating System:** Windows 10 (Build 19041+) or Windows 11 (64-bit)
- **Zig Toolchain:** Zig `0.17.1` or later
- **Python:** Python `3.10+` (CPython with `python.exe` in PATH)
- **Terminal:** PowerShell 7+ (recommended) or Windows PowerShell 5.1 / Windows Terminal

---

## 🛠️ Step 1: Install Prerequisites

### 1. Install Zig
You can install Zig using either `winget` or direct download:

#### Option A: Using Windows Package Manager (`winget`)
Open PowerShell as Administrator and run:
```powershell
winget install -e --id zig.zig
```

#### Option B: Manual Download
1. Download the Windows `x86_64` zip from the [Zig Downloads Page](https://ziglang.org/download/).
2. Extract the archive to `C:\tools\zig` (or a directory of your choice).
3. Add `C:\tools\zig` to your User or System `PATH` environment variable:
   ```powershell
   [Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\tools\zig", "User")
   ```

Verify the installation:
```powershell
zig version
# Should print 0.17.1 or higher
```

---

### 2. Install Python
Python powers the server worker and AI/data processing routines.

#### Option A: Using `winget`
```powershell
winget install -e --id Python.Python.3.13
```

#### Option B: Official Python Installer
1. Download from [python.org](https://www.python.org/downloads/windows/).
2. **Crucial:** Ensure the checkbox **"Add python.exe to PATH"** is checked on the first setup screen!

Verify the installation:
```powershell
python --version
# Should print Python 3.10.x, 3.11.x, 3.12.x, or 3.13.x
```

---

## 🚀 Step 2: Build & Install Zvelte on Windows

### Build from Source
Open PowerShell in your working directory:

```powershell
# 1. Clone the repository
git clone https://github.com/babar-xagi/Zvelte.git
cd Zvelte

# 2. Build the optimized Windows executable
zig build -Doptimize=ReleaseFast

# 3. Create a local bin folder (if it doesn't exist) and copy zvelte.exe
$binPath = "$HOME\.local\bin"
if (!(Test-Path $binPath)) { New-Item -ItemType Directory -Path $binPath }
Copy-Item ".\zig-out\bin\zvelte.exe" "$binPath\zvelte.exe"

# 4. Add the local bin folder to your User PATH
if ($env:Path -notlike "*$binPath*") {
    [Environment]::SetEnvironmentVariable("Path", $env:Path + ";$binPath", "User")
    $env:Path += ";$binPath"
}

# 5. Verify Zvelte
zvelte --version
# Output: zvelte version 0.0.4
```

---

## 🎯 Step 3: Create & Run Your First App

### 1. Scaffold a New Project
```powershell
zvelte new myapp
cd myapp
```
Zvelte guarantees a strict **two-file layout**:
```text
myapp/
├── app.toml      # Project configuration
└── app.zvelte    # Single-file component
```

### 2. Start the Development Server
```powershell
zvelte dev
```
You will see:
```text
✔ Python server worker supervisor online
✔ Zvelte dev server running at http://127.0.0.1:3000/
```
Open [http://127.0.0.1:3000/](http://127.0.0.1:3000/) in your browser (Edge, Chrome, Brave, Firefox).

### 3. Production Build
```powershell
zvelte build
```
Generates an optimized standalone distribution in `dist/`:
- `dist/index.html` — Static HTML shell
- `dist/app.js` — Fine-grained reactive UI runtime and RPC bridge
- `dist/worker.py` — Python server worker
- `dist/libnative.so` / `native.dll` — Bare-metal Zig C-ABI shared library

---

## 🔧 Step 4: Troubleshooting Common Windows Issues

### Issue 1: `error: BindFailed` on Windows
- **Cause in versions < 0.0.4:** Early dev server builds used Linux-specific socket syscalls.
- **Fix in v0.0.4+:** The dev server now uses Zig's cross-platform `std.Io.net` stack backed by Windows Winsock (`ws2_32`).
- **If Port 3000 is occupied:** Zvelte `v0.0.4+` automatically detects if port 3000 is taken and automatically switches to `3001`, `3002`, etc.
- **Manual override:** Edit `[dev]` in `app.toml`:
  ```toml
  [dev]
  host = "127.0.0.1"
  port = 3001   # Change to any open port
  ```

### Issue 2: Python Command Not Found (`python3` vs `python`)
- On Windows, Python is typically installed as `python.exe` (not `python3.exe`).
- Zvelte `v0.0.4+` automatically detects `python` when `python3` is not found.
- If neither is found, check that your Python directory (e.g. `C:\Users\<User>\AppData\Local\Programs\Python\Python313`) is in your Windows `PATH`.

### Issue 3: UTF-8 Box Characters in PowerShell (`Γ£ö`, `Γö£`)
If your PowerShell terminal displays scrambled characters for Unicode symbols like `✔` or `├──`:
```powershell
# Set console encoding to UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
chcp 65001
```
Tip: Use **Windows Terminal** (available in the Microsoft Store), which supports UTF-8, ANSI color codes, and modern fonts by default.
