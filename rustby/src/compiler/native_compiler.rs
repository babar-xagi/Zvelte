use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeParam {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFunction {
    pub name: String,
    pub params: Vec<NativeParam>,
    pub return_type: String,
}

pub fn parse_exported_functions(code: &str) -> Vec<NativeFunction> {
    let mut funcs = Vec::new();

    for raw_line in code.lines() {
        let line = raw_line.trim();
        // Match: [pub] extern "C" fn <name>(<params>) [-> <ret>]
        if !line.contains("extern \"C\" fn ") && !line.contains("extern fn ") {
            continue;
        }

        let fn_idx = match line.find("fn ") {
            Some(i) => i + 3,
            None => continue,
        };
        let after_fn = &line[fn_idx..];

        let paren_open = match after_fn.find('(') {
            Some(i) => i,
            None => continue,
        };
        let fn_name = after_fn[..paren_open].trim().to_string();

        let paren_close = match after_fn.find(')') {
            Some(i) => i,
            None => continue,
        };
        let params_str = &after_fn[paren_open + 1..paren_close];

        let mut params = Vec::new();
        for param_chunk in params_str.split(',') {
            let p_trim = param_chunk.trim();
            if p_trim.is_empty() {
                continue;
            }
            if let Some(colon_idx) = p_trim.find(':') {
                let p_name = p_trim[..colon_idx].trim().to_string();
                let p_type = p_trim[colon_idx + 1..].trim().to_string();
                params.push(NativeParam {
                    name: p_name,
                    type_name: p_type,
                });
            }
        }

        let after_paren = &after_fn[paren_close + 1..];
        let ret_type = if let Some(arrow_idx) = after_paren.find("->") {
            let after_arrow = after_paren[arrow_idx + 2..].trim();
            if let Some(brace_idx) = after_arrow.find('{') {
                after_arrow[..brace_idx].trim().to_string()
            } else {
                after_arrow.trim().to_string()
            }
        } else if let Some(brace_idx) = after_paren.find('{') {
            let mid = after_paren[..brace_idx].trim();
            if mid.is_empty() { "void".to_string() } else { mid.to_string() }
        } else {
            "void".to_string()
        };

        funcs.push(NativeFunction {
            name: fn_name,
            params,
            return_type: if ret_type.is_empty() || ret_type == "()" { "void".to_string() } else { ret_type },
        });
    }

    funcs
}

pub fn generate_ctypes_bindings(funcs: &[NativeFunction]) -> String {
    let mut out = String::new();

    out.push_str(
        r#"# === Zvelte Native Rust C-ABI ctypes Loader ===
import ctypes
import os
import sys

_native_lib = None
_lib_names = ["libnative.so", "libnative.dylib", "native.dll"]
for _name in _lib_names:
    _candidate = os.path.join(os.path.dirname(__file__), _name)
    if os.path.exists(_candidate):
        try:
            _native_lib = ctypes.CDLL(_candidate)
            break
        except Exception:
            pass

def _get_ctypes_type(t_name):
    m = {
        "i32": ctypes.c_int32,
        "i64": ctypes.c_int64,
        "isize": ctypes.c_ssize_t,
        "u32": ctypes.c_uint32,
        "u64": ctypes.c_uint64,
        "usize": ctypes.c_size_t,
        "f32": ctypes.c_float,
        "f64": ctypes.c_double,
        "bool": ctypes.c_bool,
        "void": None,
    }
    return m.get(t_name, ctypes.c_int32)
"#,
    );

    for f in funcs {
        let name = &f.name;
        out.push_str(&format!(
            "\nif _native_lib and hasattr(_native_lib, \"{}\"):\n",
            name
        ));
        out.push_str(&format!(
            "    _func_{} = getattr(_native_lib, \"{}\")\n",
            name, name
        ));

        // argtypes
        let mut argtypes_str = String::new();
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                argtypes_str.push_str(", ");
            }
            argtypes_str.push_str(&format!("_get_ctypes_type(\"{}\")", p.type_name));
        }
        out.push_str(&format!(
            "    _func_{}.argtypes = [{}]\n",
            name, argtypes_str
        ));
        out.push_str(&format!(
            "    _func_{}.restype = _get_ctypes_type(\"{}\")\n",
            name, f.return_type
        ));

        // python wrapper
        out.push_str(&format!("    def {}(*args):\n", name));
        out.push_str("        c_args = []\n");
        out.push_str(&format!("        for i, a in enumerate(args):\n"));
        out.push_str(&format!("            if i < len(_func_{}.argtypes) and _func_{}.argtypes[i] in (ctypes.c_int32, ctypes.c_int64, ctypes.c_uint32, ctypes.c_uint64, ctypes.c_size_t, ctypes.c_ssize_t):\n", name, name));
        out.push_str("                c_args.append(int(a))\n");
        out.push_str(&format!("            elif i < len(_func_{}.argtypes) and _func_{}.argtypes[i] in (ctypes.c_float, ctypes.c_double):\n", name, name));
        out.push_str("                c_args.append(float(a))\n");
        out.push_str("            else:\n");
        out.push_str("                c_args.append(a)\n");
        out.push_str(&format!("        return _func_{}(*c_args)\n", name));
        out.push_str("else:\n");
        out.push_str(&format!("    def {}(*args):\n", name));
        out.push_str(&format!(
            "        raise RuntimeError(\"Native function '{}' not available (libnative.so not loaded)\")\n",
            name
        ));
    }

    out
}

fn resolve_rustc_cmd() -> String {
    if let Ok(r) = std::env::var("RUSTC") {
        if !r.trim().is_empty() {
            return r;
        }
    }
    if Command::new("rustc").arg("--version").output().is_ok() {
        return "rustc".to_string();
    }
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        let p = Path::new(&cargo_home).join("bin").join("rustc");
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = Path::new(&home).join(".cargo").join("bin").join("rustc");
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    if let Ok(userprof) = std::env::var("USERPROFILE") {
        let p = Path::new(&userprof).join(".cargo").join("bin").join("rustc.exe");
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    "rustc".to_string()
}

pub fn compile_native_library(out_dir: &Path, code: &str) -> Result<usize, String> {
    let native_src_path = out_dir.join("native.rs");
    std::fs::write(&native_src_path, code)
        .map_err(|e| format!("failed to write native.rs: {}", e))?;

    let lib_name = if cfg!(target_os = "windows") {
        "native.dll"
    } else if cfg!(target_os = "macos") {
        "libnative.dylib"
    } else {
        "libnative.so"
    };

    let rustc_cmd = resolve_rustc_cmd();
    let output = Command::new(&rustc_cmd)
        .current_dir(out_dir)
        .args(&[
            "--crate-type",
            "cdylib",
            "-O",
            "native.rs",
            "-o",
            lib_name,
        ])
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                let err_str = String::from_utf8_lossy(&out.stderr);
                return Err(format!("rustc --crate-type cdylib failed: {}", err_str));
            }
            let lib_path = out_dir.join(lib_name);
            let metadata = std::fs::metadata(&lib_path)
                .map_err(|e| format!("cannot stat {}: {}", lib_name, e))?;
            Ok(metadata.len() as usize)
        }
        Err(e) => Err(format!("failed to execute rustc compiler: {}", e)),
    }
}
