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
mode = "python"
renderer = "js"
strict = true

[python]
enabled = true
executable = "python3"
version = "3.10+"

[tailwind]
enabled = true
preflight = true
dark_mode = "media"
mode = "auto"

[dev]
host = "127.0.0.1"
port = 3000
hot_reload = true
open_browser = false

[build]
output = "dist"
cache = ".zvelte"
optimization = "release"
minify = false
source_maps = true
"#,
        project_name
    )
}

pub const APP_ZVELTE_TEMPLATE: &str = r#"<script lang="python">
name = "Babar"
count = 0

def greeting(user):
    return f"Hello {user}! Greetings from Python server."

def calculate_square(n):
    return int(n) ** 2
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
        <p>Square of {count} is: <strong>{calculate_square(count)}</strong></p>
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
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}
h1 {
    color: #1e293b;
    margin-bottom: 0.5rem;
}
p {
    color: #64748b;
    margin-bottom: 1.5rem;
}
.card {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 1rem;
}
button {
    background: #0284c7;
    color: white;
    border: none;
    padding: 0.625rem 1.25rem;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    margin: 0 0.25rem;
    transition: background 0.15s;
}
button:hover {
    background: #0369a1;
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
