use std::path::Path;
use crate::source::ast::*;
use crate::source::parser::parse_component;
use crate::config::toml_parser::parse_app_toml;
use crate::semantic::analyzer::analyze;
use super::js_codegen::JsCodegen;
use super::native_compiler::{compile_native_library, generate_ctypes_bindings, parse_exported_functions};

pub struct BuildResult {
    pub output_dir: String,
    pub html_bytes: usize,
    pub js_bytes: usize,
    pub worker_bytes: usize,
    pub native_bytes: usize,
    pub tailwind_utilities: usize,
}

pub fn generate_python_worker(user_code: &str, ctypes_code: &str) -> String {
    format!(
        r#"#!/usr/bin/env python3
import sys
import json
import traceback

{}

# === Zvelte User Python Server Logic ===
{}
# =======================================

def handle_rpc(payload):
    method = payload.get("method")
    params = payload.get("params", [])
    if not method:
        return {{"error": "Missing 'method' in RPC request"}}
    func = globals().get(method)
    if not func or not callable(func):
        return {{"error": f"Function '{{method}}' not found on Python server"}}
    try:
        if isinstance(params, list):
            res = func(*params)
        elif isinstance(params, dict):
            res = func(**params)
        else:
            res = func(params)
        return {{"result": res}}
    except Exception as e:
        return {{"error": str(e), "traceback": traceback.format_exc()}}

def main():
    try:
        while True:
            line = sys.stdin.readline()
            if not line:
                break
            line = line.strip()
            if not line:
                continue
            try:
                req = json.loads(line)
                res = handle_rpc(req)
                if "id" in req:
                    res["id"] = req["id"]
                sys.stdout.write(json.dumps(res) + "\n")
                sys.stdout.flush()
            except Exception as err:
                sys.stdout.write(json.dumps({{"error": str(err)}}) + "\n")
                sys.stdout.flush()
    except KeyboardInterrupt:
        sys.exit(0)

if __name__ == "__main__":
    main()
"#,
        ctypes_code, user_code
    )
}

pub fn build_project(project_dir: &Path, output_dir_name: &str) -> Result<BuildResult, String> {
    // 1. Read app.toml
    let toml_path = project_dir.join("app.toml");
    let toml_content = std::fs::read_to_string(&toml_path)
        .map_err(|e| format!("failed to read app.toml: {}", e))?;
    let config = parse_app_toml(&toml_content)
        .map_err(|e| format!("invalid app.toml: {}", e))?;

    // 2. Read entry component
    let entry_path = project_dir.join(&config.project.entry);
    let entry_content = std::fs::read_to_string(&entry_path)
        .map_err(|e| format!("failed to read {}: {}", config.project.entry, e))?;

    // 3. Parse component
    let comp = parse_component(&entry_content)
        .map_err(|e| format!("parse error in {}: {}", config.project.entry, e))?;

    // 4. Semantic analysis
    let _ = analyze(comp.clone(), &config)
        .map_err(|e| format!("semantic error: {}", e))?;

    // 5. Create output directory
    let out_dir = project_dir.join(output_dir_name);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output dir {:?}: {}", out_dir, e))?;

    // 6. Generate HTML (with integrated Tailwind CSS compiler) and JS
    let tw_count = crate::compiler::tailwind::compile_tailwind(&comp, &config.tailwind, Some(project_dir))?
        .utility_count;
    let mut js_gen = JsCodegen::new(&comp, &config.project.name);
    let codegen_out = js_gen.generate_with_tailwind(&config.tailwind, Some(project_dir))?;

    let html_path = out_dir.join("index.html");
    std::fs::write(&html_path, &codegen_out.html)
        .map_err(|e| format!("failed to write index.html: {}", e))?;

    let js_path = out_dir.join("app.js");
    std::fs::write(&js_path, &codegen_out.js)
        .map_err(|e| format!("failed to write app.js: {}", e))?;

    // 7. Check for native Rust script blocks & compile cdylib via rustc
    let mut native_bytes = 0;
    let mut ctypes_code = String::new();
    let mut has_native = false;

    for script in &comp.scripts {
        if script.lang == ScriptLang::Rust && script.context == ScriptContext::Native {
            has_native = true;
            match compile_native_library(&out_dir, &script.code) {
                Ok(bytes) => native_bytes = bytes,
                Err(e) => eprintln!("  ! Warning: could not run 'rustc' ({})", e),
            }
            let native_funcs = parse_exported_functions(&script.code);
            ctypes_code = generate_ctypes_bindings(&native_funcs);
            break;
        }
    }

    // 8. Write worker.py if python server scripts exist or if native Rust exists
    let mut worker_bytes = 0;
    for script in &comp.scripts {
        if script.lang == ScriptLang::Python && script.context == ScriptContext::Server {
            let worker_code = generate_python_worker(&script.code, &ctypes_code);
            let worker_path = out_dir.join("worker.py");
            std::fs::write(&worker_path, &worker_code)
                .map_err(|e| format!("failed to write worker.py: {}", e))?;
            worker_bytes = worker_code.len();
            break;
        }
    }

    if worker_bytes == 0 && has_native {
        let worker_code = generate_python_worker("", &ctypes_code);
        let worker_path = out_dir.join("worker.py");
        std::fs::write(&worker_path, &worker_code)
            .map_err(|e| format!("failed to write worker.py: {}", e))?;
        worker_bytes = worker_code.len();
    }

    Ok(BuildResult {
        output_dir: output_dir_name.to_string(),
        html_bytes: codegen_out.html.len(),
        js_bytes: codegen_out.js.len(),
        worker_bytes,
        native_bytes,
        tailwind_utilities: tw_count,
    })
}
