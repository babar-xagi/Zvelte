use std::path::Path;

pub fn validate_project_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 {
        return false;
    }
    if name == "." || name == ".." {
        return false;
    }
    for c in name.chars() {
        if c == '/' || c == '\\' {
            return false;
        }
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return false;
        }
    }
    true
}

pub fn get_app_toml_template(project_name: &str) -> String {
    format!(
        r#"[project]
name = "{}"
version = "0.1.0"
entry = "app.zvelte"

[app]
target = "web"
mode = "hybrid"              # "python", "rust", or "hybrid"
renderer = "js"               # "js", "wasm", or "auto"
strict = true

[toolchain]
rust = "1.80+"
python = "3.10+"

[python]
enabled = true
executable = "python3"
execution = "server"          # CPython server-side; not browser Python
rust_acceleration = true

[rust]
enabled = true
executable = "rustc"
native_modules = true
wasm_modules = true

[dev]
host = "127.0.0.1"
port = 3000
hot_reload = true
open_browser = false

[build]
output = "dist"
cache = ".rustby"
optimization = "release"
minify = false
source_maps = true
"#,
        project_name
    )
}

pub const APP_ZVELTE_TEMPLATE: &str = r#"<script lang="rust">
#[no_mangle]
pub extern "C" fn rust_fast_fib(n: u32) -> u64 {
    if n <= 1 { return n as u64; }
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}
</script>

<script lang="python">
name = "Babar"
count = 0

def greeting(user):
    return f"Hello {user}! Greetings from Python server worker."

def benchmark_rust_fib(n):
    val = rust_fast_fib(int(n))
    return f"Rust Native Fibonacci({n}) = {val} (sub-microsecond execution)"
</script>

<main>
    <h1>Hello {name}</h1>
    <p>{greeting(name)}</p>

    <div class="card">
        <button click={count++}>
            Clicked {count} times
        </button>
        <button click={count = 0}>
            Reset
        </button>
    </div>

    <div class="card">
        <p>{benchmark_rust_fib(40)}</p>
    </div>
</main>

<style>
main {
    font-family: system-ui, -apple-system, sans-serif;
    max-width: 36rem;
    margin: 4rem auto;
    text-align: center;
    padding: 2.5rem;
    background: #ffffff;
    border-radius: 12px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
}
.card {
    margin-top: 1.5rem;
    display: flex;
    justify-content: center;
    gap: 0.75rem;
}
button {
    padding: 0.6rem 1.4rem;
    border-radius: 8px;
    border: 1px solid #0070f3;
    background: #0070f3;
    color: white;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
}
button:hover {
    background: #005bb5;
}
</style>
"#;

pub fn create_project(project_name: &str, base_dir: &Path) -> Result<(), String> {
    if !validate_project_name(project_name) {
        return Err(format!("'{}' is not a valid project name.", project_name));
    }

    let proj_dir = base_dir.join(project_name);
    if proj_dir.exists() {
        return Err(format!("directory '{}' already exists. Refusing to overwrite.", project_name));
    }

    std::fs::create_dir_all(&proj_dir)
        .map_err(|e| format!("failed to create directory: {}", e))?;

    let toml_content = get_app_toml_template(project_name);
    std::fs::write(proj_dir.join("app.toml"), toml_content)
        .map_err(|e| format!("failed to write app.toml: {}", e))?;

    std::fs::write(proj_dir.join("app.zvelte"), APP_ZVELTE_TEMPLATE)
        .map_err(|e| format!("failed to write app.zvelte: {}", e))?;

    Ok(())
}
