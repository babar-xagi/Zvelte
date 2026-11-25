use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use zvelte::cli::args::VERSION;
use zvelte::cli::run;
use zvelte::cli::scaffold::{create_project, validate_project_name};
use zvelte::compiler::codegen::{build_project, generate_python_worker};
use zvelte::compiler::js_codegen::JsCodegen;
use zvelte::compiler::native_compiler::{
    compile_native_library, generate_ctypes_bindings, parse_exported_functions,
};
use zvelte::compiler::tailwind::compile_tailwind;
use zvelte::config::toml_parser::{parse_app_toml, TomlError};
use zvelte::config::types::{AppMode, Renderer, TailwindConfig};
use zvelte::devserver::server::DevServer;
use zvelte::semantic::analyzer::{analyze, SemanticError};
use zvelte::source::ast::{NodeType, ScriptContext, ScriptLang};
use zvelte::source::parser::{parse_component, ParseError};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "zvelte_test_{}_{}_{}",
            prefix,
            std::process::id(),
            id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        Self { path: dir }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// =========================================================================
// 1. TOML Configuration Parser Tests
// =========================================================================

#[test]
fn test_toml_full_config_and_backcompat() {
    let toml = r##"
[project]
name = "demo-app"
version = "0.2.0"
entry = "main.zvelte"

[app]
target = "web"
mode = "python"
renderer = "wasm"
strict = false

[toolchain]
zig = "0.17.1"
python = "3.12"

[python]
enabled = true
executable = "python3"
execution = "server"
zig_acceleration = false

[dev]
host = "0.0.0.0"
port = 8080
hot_reload = false
open_browser = true

[build]
output = "out"
cache = ".cache"
optimization = "release-fast"
minify = true
source_maps = false
"##;

    let cfg = parse_app_toml(toml).expect("valid toml should parse");
    assert_eq!(cfg.project.name, "demo-app");
    assert_eq!(cfg.project.version, "0.2.0");
    assert_eq!(cfg.project.entry, "main.zvelte");
    assert_eq!(cfg.app.mode, AppMode::Python);
    assert_eq!(cfg.app.renderer, Renderer::Wasm);
    assert!(!cfg.app.strict);
    assert_eq!(cfg.toolchain.rust, "0.17.1");
    assert_eq!(cfg.toolchain.python, "3.12");
    assert!(!cfg.python.rust_acceleration);
    assert_eq!(cfg.dev.host, "0.0.0.0");
    assert_eq!(cfg.dev.port, 8080);
    assert!(!cfg.dev.hot_reload);
    assert!(cfg.dev.open_browser);
    assert_eq!(cfg.build.output, "out");
    assert!(cfg.build.minify);
    assert!(!cfg.build.source_maps);
}

#[test]
fn test_toml_invalid_mode_renderer_port_errors() {
    let bad_mode = "[app]\nmode = \"ruby\"\n";
    assert_eq!(
        parse_app_toml(bad_mode),
        Err(TomlError::InvalidValue("ruby".to_string()))
    );

    let bad_renderer = "[app]\nrenderer = \"opengl\"\n";
    assert_eq!(
        parse_app_toml(bad_renderer),
        Err(TomlError::InvalidValue("opengl".to_string()))
    );

    let bad_port = "[dev]\nport = not_a_number\n";
    assert_eq!(
        parse_app_toml(bad_port),
        Err(TomlError::InvalidValue("not_a_number".to_string()))
    );
}

#[test]
fn test_toml_hash_inside_quoted_string_claim() {
    // Quoted strings in TOML containing '#' should NOT be truncated as comments
    let toml = r##"
[project]
name = "Color #ff0000 App"
"##;
    let cfg = parse_app_toml(toml).expect("parse toml");
    assert_eq!(cfg.project.name, "Color #ff0000 App");
}

// =========================================================================
// 2. Component (.zvelte) Parser Tests
// =========================================================================

#[test]
fn test_parser_script_defaults_and_contexts() {
    let src = r#"
<script>
let x = 1;
</script>
<script lang="python">
y = 2
</script>
<script lang="rust">
pub extern "C" fn add(a: i32, b: i32) -> i32 { a + b }
</script>
<script context="server">
z = "hello"
</script>
"#;
    let comp = parse_component(src).expect("should parse all script variants");
    assert_eq!(comp.scripts.len(), 4);
    assert_eq!(comp.scripts[0].lang, ScriptLang::Js);
    assert_eq!(comp.scripts[0].context, ScriptContext::Client);
    assert_eq!(comp.scripts[1].lang, ScriptLang::Python);
    assert_eq!(comp.scripts[1].context, ScriptContext::Server);
    assert_eq!(comp.scripts[2].lang, ScriptLang::Rust);
    assert_eq!(comp.scripts[2].context, ScriptContext::Native);
    assert_eq!(comp.scripts[3].lang, ScriptLang::Python);
    assert_eq!(comp.scripts[3].context, ScriptContext::Server);

    assert_eq!(comp.states.len(), 3);
    assert_eq!(comp.states[0].name, "x");
    assert_eq!(comp.states[1].name, "y");
    assert_eq!(comp.states[2].name, "z");
}

#[test]
fn test_parser_if_else_each_bindings_and_void_elements() {
    let src = r#"
<state>
items: []string = ["a", "b"]
active: bool = true
name: string = ""
</state>

<main>
  <input type="text" bind:value={name}>
  <input type="checkbox" bind:checked={active} />
  <br>
  {#if active}
    <p class="ok">Yes: {name}</p>
  {:else}
    <p>No</p>
  {/if}
  {#each items as item (item)}
    <span>{item}</span>
  {/each}
</main>
<style>
main { color: red; }
</style>
"#;
    let comp = parse_component(src).expect("should parse template blocks");
    assert_eq!(comp.states.len(), 3);
    assert_eq!(comp.styles, "main { color: red; }");
    assert_eq!(comp.root_nodes.len(), 1);
    let main_el = &comp.root_nodes[0];
    assert_eq!(main_el.tag, "main");

    let elements: Vec<_> = main_el
        .children
        .iter()
        .filter(|n| n.node_type != NodeType::Text || !n.content.trim().is_empty())
        .collect();
    assert_eq!(elements.len(), 5);
    assert_eq!(elements[0].bindings[0].property, "value");
    assert_eq!(elements[0].bindings[0].variable, "name");
    assert_eq!(elements[1].bindings[0].property, "checked");
    assert_eq!(elements[1].bindings[0].variable, "active");
    assert_eq!(elements[2].tag, "br");
    assert_eq!(elements[3].node_type, NodeType::IfBlock);
    assert_eq!(elements[3].condition.as_deref(), Some("active"));
    assert!(!elements[3].alternate.is_empty());
    assert_eq!(elements[4].node_type, NodeType::EachBlock);
    assert_eq!(elements[4].collection.as_deref(), Some("items"));
    assert_eq!(elements[4].item_name.as_deref(), Some("item"));
    assert_eq!(elements[4].key_name.as_deref(), Some("item"));
}

#[test]
fn test_parser_error_cases() {
    assert_eq!(
        parse_component("<div><span></div>"),
        Err(ParseError::MismatchedTag {
            expected: "span".to_string(),
            found: "div".to_string()
        })
    );
    assert_eq!(
        parse_component("<script lang=\"python\">x = 1"),
        Err(ParseError::UnterminatedScript)
    );
    assert_eq!(
        parse_component("<state>count: i32 = 0"),
        Err(ParseError::UnterminatedState)
    );
    assert_eq!(
        parse_component("<style>body {}"),
        Err(ParseError::UnterminatedStyle)
    );
    assert_eq!(
        parse_component("<state>\ncount = 0\n</state>"),
        Err(ParseError::InvalidStateDeclaration("count = 0".to_string()))
    );
    assert_eq!(
        parse_component("{#if true}<p>hi</p>"),
        Err(ParseError::UnterminatedIfBlock)
    );
    assert_eq!(
        parse_component("{#each items as item}<p>{item}</p>"),
        Err(ParseError::UnterminatedEachBlock)
    );
    assert!(matches!(
        parse_component("{#each items}<p>x</p>{/each}"),
        Err(ParseError::InvalidEachSyntax(_))
    ));
}

#[test]
fn test_parser_html_comments_claim_in_readme_and_docs() {
    // README.md line 28 and docs/COMPONENT_FORMAT.md line 12 & 204 show HTML comments in .zvelte files
    let src = r#"
<!-- app.zvelte -->
<main>
  <!-- Increment state -->
  <p>Hello</p>
</main>
"#;
    let comp = parse_component(src).expect("HTML comments <!-- ... --> documented in README/docs should parse cleanly");
    assert!(!comp.root_nodes.is_empty());
}

#[test]
fn test_python_server_non_literal_vars_must_not_leak_to_client_js() {
    // README.md claim: "Zero API Key Leakage: GEMINI_API_KEY stays strictly in the isolated
    // CPython worker process. It is never bundled or exposed in browser JavaScript assets."
    let src = r#"
<script lang="python">
import os
GEMINI_KEY = os.getenv("GEMINI_API_KEY", "")
OPENAI_KEY = os.getenv("OPENAI_API_KEY", "")
public_greeting = "Hello"
</script>
<main><p>{public_greeting}</p></main>
"#;
    let comp = parse_component(src).expect("parse python server script");
    let mut codegen = JsCodegen::new(&comp, "ai-test");
    let js = codegen.generate_js();
    assert!(
        !js.contains("os.getenv"),
        "Server Python call os.getenv(...) leaked into client app.js state! Generated:\n{}",
        js
    );
    assert!(
        !js.contains("GEMINI_KEY"),
        "Server secret GEMINI_KEY leaked into client app.js state! Generated:\n{}",
        js
    );
}

// =========================================================================
// 3. Semantic Analyzer Tests
// =========================================================================

#[test]
fn test_semantic_mode_enforcement() {
    let py_cfg = parse_app_toml("[app]\nmode = \"python\"\n").unwrap();
    let rust_comp = parse_component("<script lang=\"rust\">pub extern \"C\" fn f() {}</script>").unwrap();
    assert_eq!(
        analyze(rust_comp, &py_cfg).unwrap_err(),
        SemanticError::RustBlockNotAllowedInPythonMode
    );

    let rs_cfg = parse_app_toml("[app]\nmode = \"rust\"\n").unwrap();
    let py_comp = parse_component("<script lang=\"python\">def f(): pass</script>").unwrap();
    assert_eq!(
        analyze(py_comp, &rs_cfg).unwrap_err(),
        SemanticError::PythonBlockNotAllowedInRustMode
    );
}

#[test]
fn test_semantic_state_and_expression_validation() {
    let cfg = parse_app_toml("[app]\nmode = \"hybrid\"\n").unwrap();

    // Duplicate state
    let dup = parse_component("<state>\na: i32 = 1\na: i32 = 2\n</state>").unwrap();
    assert_eq!(
        analyze(dup, &cfg).unwrap_err(),
        SemanticError::DuplicateStateVariable("a".to_string())
    );

    // Unsupported type
    let bad_type = parse_component("<state>\na: hashmap = {}\n</state>").unwrap();
    assert_eq!(
        analyze(bad_type, &cfg).unwrap_err(),
        SemanticError::UnsupportedStateType("hashmap".to_string())
    );

    // Unknown state in event
    let bad_evt = parse_component("<button click={missing++}>+</button>").unwrap();
    assert_eq!(
        analyze(bad_evt, &cfg).unwrap_err(),
        SemanticError::UnknownStateInEvent("missing".to_string())
    );

    // Unknown state in bind
    let bad_bind = parse_component("<input bind:value={missing} />").unwrap();
    assert_eq!(
        analyze(bad_bind, &cfg).unwrap_err(),
        SemanticError::UnknownStateInBinding("missing".to_string())
    );

    // Unknown collection in each
    let bad_each = parse_component("{#each missing as x}<p>{x}</p>{/each}").unwrap();
    assert_eq!(
        analyze(bad_each, &cfg).unwrap_err(),
        SemanticError::UnknownCollectionInEach("missing".to_string())
    );

    // Complex event with JS keywords, arrow callback, string literals, and state + python fn
    let ok_src = r#"
<script lang="python">
def ask(q, m): return q
</script>
<state>
prompt: string = ""
reply: string = ""
model: string = "gemini"
loading: bool = false
</state>
<button click={
    loading = true;
    model = "openai";
    const q = prompt;
    ask(q, model).then(res => { reply = res; loading = false; update(); });
}>Go</button>
"#;
    let ok_comp = parse_component(ok_src).unwrap();
    assert!(analyze(ok_comp, &cfg).is_ok());
}

// =========================================================================
// 4. JavaScript & Python Worker Codegen Tests
// =========================================================================

#[test]
fn test_js_codegen_string_literal_preservation() {
    // If a state variable is named 'gemini', setting model = "gemini" should NOT rewrite the string literal!
    let src = r#"
<state>
gemini: i32 = 1
selected: string = "none"
</state>
<button click={selected = "gemini"}>{gemini}</button>
"#;
    let comp = parse_component(src).unwrap();
    let mut codegen = JsCodegen::new(&comp, "str-test");
    let out = codegen.generate();
    assert!(
        out.js.contains("state.selected = \"gemini\""),
        "String literal \"gemini\" was wrongly rewritten! Generated JS:\n{}",
        out.js
    );
}

#[test]
fn test_js_codegen_windows_crlf_newlines() {
    // Docs explicitly support Windows 10/11 users whose files commonly have CRLF (\r\n)
    let src = "<state>\r\ncount: i32 = 0\r\n</state>\r\n<main>\r\n  <p>{count}</p>\r\n</main>";
    let comp = parse_component(src).expect("CRLF file should parse");
    let mut codegen = JsCodegen::new(&comp, "crlf-test");
    let out = codegen.generate();
    assert!(
        !out.js.contains('\r'),
        "Unescaped raw \\r in JS string literal causes JS SyntaxError! Generated JS:\n{}",
        out.js
    );
}

#[test]
fn test_native_compiler_signature_parser_and_ctypes() {
    let rust_code = r#"
#[no_mangle]
pub extern "C" fn rust_multiply(a: i32, b: i32) -> i32 {
    a * b
}

pub extern "C" fn rust_noop(flag: bool) {
}
"#;
    let funcs = parse_exported_functions(rust_code);
    assert_eq!(funcs.len(), 2);
    assert_eq!(funcs[0].name, "rust_multiply");
    assert_eq!(funcs[0].params.len(), 2);
    assert_eq!(funcs[0].params[0].type_name, "i32");
    assert_eq!(funcs[0].return_type, "i32");
    assert_eq!(funcs[1].name, "rust_noop");
    assert_eq!(funcs[1].return_type, "void");

    let py = generate_ctypes_bindings(&funcs);
    assert!(py.contains("ctypes.CDLL"));
    assert!(py.contains("def rust_multiply(*args):"));
    assert!(py.contains("_func_rust_multiply.restype = _get_ctypes_type(\"i32\")"));
}

#[test]
fn test_native_cdylib_compile_if_rustc_available() {
    let tmp = TempDir::new("native_cdylib");
    let code = r#"
#[no_mangle]
pub extern "C" fn test_add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let res = compile_native_library(tmp.path(), code);
    assert!(res.is_ok(), "rustc cdylib compile failed: {:?}", res.err());
    assert!(res.unwrap() > 0);
}

#[test]
fn test_python_worker_rpc_live_execution() {
    let tmp = TempDir::new("worker_rpc");
    let worker = generate_python_worker("def greet(name):\n    return f'Hi {name}!'\n", "");
    let worker_path = tmp.path().join("worker.py");
    fs::write(&worker_path, worker).unwrap();

    let mut child = Command::new("python3")
        .arg("-u")
        .arg(&worker_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn python3 worker");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(br#"{"jsonrpc":"2.0","id":7,"method":"greet","params":["Zvelte"]}"#)
            .unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    std::io::BufRead::read_line(&mut stdout, &mut line).unwrap();
    let _ = child.kill();
    let _ = child.wait();

    assert!(line.contains(r#""result": "Hi Zvelte!""#), "got: {}", line);
    assert!(line.contains(r#""id": 7"#), "got: {}", line);
}

// =========================================================================
// 5. CLI & Two-File Scaffold Tests
// =========================================================================

#[test]
fn test_project_name_validation_and_two_file_contract() {
    assert!(validate_project_name("my-app_01"));
    assert!(!validate_project_name(""));
    assert!(!validate_project_name("."));
    assert!(!validate_project_name(".."));
    assert!(!validate_project_name("../escape"));
    assert!(!validate_project_name("foo/bar"));
    assert!(!validate_project_name("my app"));

    let tmp = TempDir::new("scaffold");
    create_project("two-file-app", tmp.path()).expect("scaffold must succeed");

    let proj_dir = tmp.path().join("two-file-app");
    let entries: Vec<_> = fs::read_dir(&proj_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&"app.toml".to_string()));
    assert!(entries.contains(&"app.zvelte".to_string()));

    // Refuses overwrite
    assert!(create_project("two-file-app", tmp.path()).is_err());

    // CLI build + check + clean workflow preserves two-file contract
    let proj_str = proj_dir.to_string_lossy().to_string();
    assert!(run(&["zvelte".into(), "check".into(), proj_str.clone()]).is_ok());
    assert!(run(&["zvelte".into(), "build".into(), proj_str.clone()]).is_ok());
    assert!(proj_dir.join("dist/index.html").exists());
    assert!(proj_dir.join("dist/app.js").exists());
    assert!(proj_dir.join("dist/worker.py").exists());

    assert!(run(&["zvelte".into(), "clean".into(), proj_str]).is_ok());
    let entries_after_clean: Vec<_> = fs::read_dir(&proj_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries_after_clean.len(), 2);
}

#[test]
fn test_cli_version_help_and_unknown_command() {
    assert!(!VERSION.is_empty());
    assert!(run(&["zvelte".into(), "--version".into()]).is_ok());
    assert!(run(&["zvelte".into(), "--help".into()]).is_ok());
    assert!(run(&["zvelte".into(), "nonexistent-cmd".into()]).is_err());
}

#[test]
fn test_all_repo_examples_check_and_build() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let examples = [
        "01-counter",
        "02-todo-app",
        "03-python-api",
        "04-ai-chatbot",
        "05-tailwind-dashboard",
    ];

    for ex in examples {
        let ex_dir = manifest_dir.join("examples").join(ex);
        let tmp = TempDir::new(&format!("ex_{}", ex));
        fs::copy(ex_dir.join("app.toml"), tmp.path().join("app.toml")).unwrap();
        fs::copy(ex_dir.join("app.zvelte"), tmp.path().join("app.zvelte")).unwrap();

        let dir_str = tmp.path().to_string_lossy().to_string();
        run(&["zvelte".into(), "check".into(), dir_str.clone()])
            .unwrap_or_else(|e| panic!("{} failed check: {}", ex, e));
        let res = build_project(tmp.path(), "dist")
            .unwrap_or_else(|e| panic!("{} failed build: {}", ex, e));
        assert!(res.html_bytes > 0);
        assert!(res.js_bytes > 0);
        if ex == "05-tailwind-dashboard" {
            assert!(
                res.tailwind_utilities > 30,
                "expected 30+ compiled Tailwind utilities in 05-tailwind-dashboard, got {}",
                res.tailwind_utilities
            );
        }
    }
}

// =========================================================================
// 6. DevServer Port Fallback & Live HTTP/RPC Serving Tests
// =========================================================================

#[test]
fn test_devserver_port_fallback_and_http_rpc() {
    let tmp = TempDir::new("devserver");
    create_project("srvapp", tmp.path()).expect("create project");
    let proj_dir = tmp.path().join("srvapp");

    // Occupy a free ephemeral port to force fallback
    let blocker = TcpListener::bind("127.0.0.1:0").unwrap();
    let occupied_port = blocker.local_addr().unwrap().port();

    let server = DevServer::new(&proj_dir, "127.0.0.1", occupied_port)
        .expect("DevServer should fall back to next port");
    assert_ne!(server.port(), occupied_port);
    assert!(server.port() > occupied_port && server.port() < occupied_port + 10);

    drop(server);
    drop(blocker);

    // Find a free port and start server handling 5 requests:
    // 1. GET /  2. GET /app.js  3. GET /__zvelte_version  4. POST /api/rpc  5. GET /nonexistent
    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let proj_clone = proj_dir.clone();
    let handle = thread::spawn(move || {
        let mut srv = DevServer::new(&proj_clone, "127.0.0.1", port).expect("bind server");
        srv.serve(Some(5)).expect("serve 5 requests");
    });

    // Retry connect until server is listening
    let mut connected = None;
    for _ in 0..50 {
        if let Ok(s) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            connected = Some(s);
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    // Request 1: GET / (also checks hot reload polling script injected in dev mode)
    {
        let mut s = connected.expect("should connect to DevServer");
        s.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"), "GET / failed: {}", resp);
        assert!(resp.contains("text/html"));
        assert!(resp.contains("<div id=\"app\"></div>"));
        assert!(resp.contains("/__zvelte_version"));
    }

    // Request 2: GET /app.js
    {
        let mut s = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        s.write_all(b"GET /app.js HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("application/javascript"));
        assert!(resp.contains("calculate_square"));
    }

    // Request 3: GET /__zvelte_version
    {
        let mut s = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        s.write_all(b"GET /__zvelte_version HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.trim_end().ends_with("1"));
    }

    // Request 4: POST /api/rpc with lowercase content-length header
    {
        let mut s = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        let body = r#"{"method":"calculate_square","params":[9]}"#;
        let req = format!(
            "POST /api/rpc HTTP/1.1\r\nHost: localhost\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"), "RPC failed: {}", resp);
        assert!(resp.contains(r#""result": 81"#), "RPC body unexpected: {}", resp);
    }

    // Request 5: GET /nonexistent -> 404
    {
        let mut s = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        s.write_all(b"GET /nonexistent HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 404 Not Found"));
    }

    handle.join().expect("server thread finishes after 5 requests");
}

#[test]
fn test_devserver_live_hot_reload_on_file_change() {
    let tmp = TempDir::new("hot_reload");
    create_project("hrapp", tmp.path()).expect("create project");
    let proj_dir = tmp.path().join("hrapp");

    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let proj_clone = proj_dir.clone();
    let handle = thread::spawn(move || {
        let mut srv = DevServer::new(&proj_clone, "127.0.0.1", port).expect("bind server");
        srv.serve(Some(3)).expect("serve 3 requests");
    });

    // Request 1: initial version == 1
    let mut connected = None;
    for _ in 0..50 {
        if let Ok(s) = TcpStream::connect(format!("127.0.0.1:{}", port)) {
            connected = Some(s);
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    {
        let mut s = connected.expect("connect to server");
        s.write_all(b"GET /__zvelte_version HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.trim_end().ends_with("1"), "expected v1, got: {}", resp);
    }

    // Sleep briefly to ensure filesystem mtime advances reliably
    thread::sleep(Duration::from_millis(50));

    // Modify app.zvelte with a new Python function & updated markup
    let updated_zvelte = r#"<script lang="python">
msg = "HotReloaded"
def get_v2():
    return "v2-active"
</script>
<main><h1>{msg}</h1><p>{get_v2()}</p></main>
"#;
    fs::write(proj_dir.join("app.zvelte"), updated_zvelte).unwrap();
    // Wait past the 200ms watcher poll window
    thread::sleep(Duration::from_millis(300));

    // Request 2: version incremented to 2
    {
        let mut s = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        s.write_all(b"GET /__zvelte_version HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.trim_end().ends_with("2"), "expected v2 after edit, got: {}", resp);
    }

    // Request 3: new Python RPC function works live on restarted worker
    {
        let mut s = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        let body = r#"{"method":"get_v2","params":[]}"#;
        let req = format!(
            "POST /api/rpc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut resp = String::new();
        s.read_to_string(&mut resp).unwrap();
        assert!(resp.contains(r#""result": "v2-active""#), "reloaded worker RPC failed: {}", resp);
    }

    handle.join().expect("hot reload server finishes");
}

#[test]
fn test_custom_build_output_dir_and_brace_quotes() {
    let tmp = TempDir::new("custom_out");
    let toml = r#"[project]
name = "custom-out"
version = "0.1.0"
entry = "app.zvelte"

[app]
mode = "python"

[build]
output = "public_out"
cache = ".mycache"
"#;
    let zvelte = r#"
<state>
msg: string = "init"
</state>
<button click={msg = "a } b"}>{msg}</button>
"#;
    fs::write(tmp.path().join("app.toml"), toml).unwrap();
    fs::write(tmp.path().join("app.zvelte"), zvelte).unwrap();

    let dir_str = tmp.path().to_string_lossy().to_string();
    run(&["zvelte".into(), "build".into(), dir_str.clone()]).expect("custom build succeeds");
    assert!(tmp.path().join("public_out/index.html").exists());
    assert!(tmp.path().join("public_out/app.js").exists());
    let js = fs::read_to_string(tmp.path().join("public_out/app.js")).unwrap();
    assert!(js.contains("state.msg = \"a } b\""));
    assert!(js.contains("__clear_scope"));

    run(&["zvelte".into(), "clean".into(), dir_str]).expect("custom clean succeeds");
    assert!(!tmp.path().join("public_out").exists());
    assert!(tmp.path().join("app.toml").exists());
    assert!(tmp.path().join("app.zvelte").exists());
}

// =========================================================================
// 7. Full Tailwind CSS Engine & Integration Tests (v0.0.7)
// =========================================================================

#[test]
fn test_tailwind_config_parsing() {
    let toml = r#"
[project]
name = "tw-app"

[tailwind]
enabled = true
preflight = false
dark_mode = "class"
mode = "builtin"
"#;
    let cfg = parse_app_toml(toml).expect("tailwind toml should parse");
    assert!(cfg.tailwind.enabled);
    assert!(!cfg.tailwind.preflight);
    assert_eq!(cfg.tailwind.dark_mode, "class");
    assert_eq!(cfg.tailwind.mode, "builtin");
    assert!(cfg.tailwind.explicit);

    let bad = "[tailwind]\ndark_mode = \"neon\"\n";
    assert_eq!(
        parse_app_toml(bad),
        Err(TomlError::InvalidValue("neon".to_string()))
    );
}

#[test]
fn test_tailwind_utilities_variants_opacity_arbitrary_and_apply() {
    let src = r#"
<state>
active: bool = true
</state>
<style>
.custom-btn {
    @apply px-4 py-2 rounded-lg font-semibold bg-sky-600 text-white;
}
</style>
<div class="group min-h-screen w-1/2 -mt-2 flex md:grid md:grid-cols-3 gap-4 bg-[#1e293b]/80 dark:md:hover:bg-indigo-600/90 !p-6 animate-pulse [mask-type:alpha]">
  <span class={active ? "bg-emerald-500 text-white" : "bg-rose-500/50 text-slate-200"}>Hi</span>
  <p class={`text-sm ${active ? 'font-bold' : 'opacity-50'}`}>Template</p>
  <i class="before:content-['*'] group-hover:opacity-100 bg-gradient-to-r from-sky-400 via-purple-500 to-indigo-600">x</i>
</div>
"#;
    let comp = parse_component(src).expect("parse tailwind component");
    let tw_cfg = TailwindConfig {
        enabled: true,
        preflight: true,
        dark_mode: "class".to_string(),
        mode: "auto".to_string(),
        explicit: true,
    };
    let out = compile_tailwind(&comp, &tw_cfg, None).expect("compile tailwind");

    // Preflight present
    assert!(out.tailwind_base.contains("@layer base"));
    // @apply expanded in custom CSS
    assert!(out.processed_custom_style.contains("padding-left: 1rem;"));
    assert!(out.processed_custom_style.contains("border-radius: 0.5rem;"));
    // Spacing, negative margin, fraction width
    assert!(out.tailwind_utilities.contains(".min-h-screen { min-height: 100vh; }"));
    assert!(out.tailwind_utilities.contains(".w-1\\/2 { width: 50%; }"));
    assert!(out.tailwind_utilities.contains(".-mt-2 { margin-top: -0.5rem; }"));
    // Important modifier
    assert!(out.tailwind_utilities.contains(".\\!p-6 { padding: 1.5rem !important; }"));
    // Arbitrary color with opacity + arbitrary property
    assert!(out.tailwind_utilities.contains("background-color: rgb(30 41 59 / 0.8);"));
    assert!(out.tailwind_utilities.contains("mask-type: alpha;"));
    // Stacked dark + md + hover + opacity
    assert!(out.tailwind_utilities.contains("@media (min-width: 768px)"));
    assert!(out.tailwind_utilities.contains(".dark .dark\\:md\\:hover\\:bg-indigo-600\\/90:hover { background-color: rgb(79 70 229 / 0.9); }"));
    // Group hover & before::
    assert!(out.tailwind_utilities.contains(".group:hover .group-hover\\:opacity-100"));
    // Dynamic expression + backtick template classes extracted
    assert!(out.tailwind_utilities.contains(".bg-emerald-500"));
    assert!(out.tailwind_utilities.contains(".bg-rose-500\\/50"));
    assert!(out.tailwind_utilities.contains(".font-bold"));
    assert!(out.tailwind_utilities.contains(".opacity-50 { opacity: 0.5; }"));
    // Gradients preserving via- and to-
    assert!(out.tailwind_utilities.contains("--tw-gradient-from: rgb(56 189 248);"));
    assert!(out.tailwind_utilities.contains("--tw-gradient-stops: var(--tw-gradient-from), rgb(168 85 247), var(--tw-gradient-to"));
    assert!(out.tailwind_utilities.contains(".to-indigo-600 { --tw-gradient-to: rgb(79 70 229); }"));
    // Keyframes emitted for animate-pulse
    assert!(out.tailwind_utilities.contains("@keyframes pulse"));
}

#[test]
fn test_tailwind_zero_overhead_when_unused_and_backtick_template_expr() {
    let src = r#"
<state>
flag: bool = true
</state>
<button class={`custom ${flag ? 'bg-sky-500' : 'bg-slate-800'}`}>Go</button>
"#;
    let comp = parse_component(src).unwrap();
    let mut codegen = JsCodegen::new(&comp, "tw-dyn");
    let out = codegen.generate();
    assert!(
        out.js.contains("${state.flag ? 'bg-sky-500' : 'bg-slate-800'}"),
        "backtick template expr inside class={{...}} should format state.flag, got:\n{}",
        out.js
    );
    assert!(out.html.contains(".bg-sky-500"));
    assert!(out.html.contains(".bg-slate-800"));

    // Non-tailwind component without explicit [tailwind] must NOT inject preflight overhead
    let plain_src = r#"<div class="my-custom-box">Hello</div>"#;
    let plain_comp = parse_component(plain_src).unwrap();
    let tw_out = compile_tailwind(&plain_comp, &TailwindConfig::default(), None).unwrap();
    assert_eq!(tw_out.utility_count, 0);
    assert!(tw_out.tailwind_base.is_empty());
    assert!(tw_out.tailwind_utilities.is_empty());
}
