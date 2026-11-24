use std::path::Path;
use crate::cli::scaffold;
use crate::config::toml_parser::parse_app_toml;
use crate::source::parser::parse_component;
use crate::semantic::analyzer::analyze;
use crate::compiler::codegen::build_project;
use crate::devserver::server::DevServer;

pub const VERSION: &str = "0.0.4";

pub fn run(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let command = args[1].as_str();

    match command {
        "--version" | "-v" | "version" => {
            println!("rustby (zvelte-rs) version {}", VERSION);
            Ok(())
        }
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        "new" => {
            if args.len() < 3 {
                return Err("'zvelte-rs new' requires a project name.\nUsage: zvelte-rs new <project-name>".to_string());
            }
            let proj_name = &args[2];
            let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
            scaffold::create_project(proj_name, &cwd)?;

            println!("\n\x1b[32m✔\x1b[0m Created Rustby (Zvelte-RS) project \x1b[1m{}\x1b[0m", proj_name);
            println!("  ├── app.toml");
            println!("  └── app.zvelte\n");
            println!("Next steps:");
            println!("  cd {}", proj_name);
            println!("  zvelte-rs dev\n");
            Ok(())
        }
        "check" => {
            let target_path = if args.len() >= 3 { &args[2] } else { "." };
            let proj_dir = Path::new(target_path);

            let toml_path = proj_dir.join("app.toml");
            let toml_source = std::fs::read_to_string(&toml_path)
                .map_err(|e| format!("failed to read app.toml: {}", e))?;
            let cfg = parse_app_toml(&toml_source)
                .map_err(|e| format!("invalid configuration in app.toml: {}", e))?;

            let zvelte_path = proj_dir.join(&cfg.project.entry);
            let zvelte_source = std::fs::read_to_string(&zvelte_path)
                .map_err(|e| format!("failed to read {}: {}", cfg.project.entry, e))?;

            let comp = parse_component(&zvelte_source)
                .map_err(|e| format!("parse error in {}: {}", cfg.project.entry, e))?;

            let _ = analyze(comp.clone(), &cfg)
                .map_err(|e| format!("semantic error: {}", e))?;

            println!(
                "\x1b[32m✔\x1b[0m Check passed: '{}' (mode: {}, renderer: {}, {} state vars, {} scripts)",
                cfg.project.name,
                cfg.app.mode.as_str(),
                cfg.app.renderer.as_str(),
                comp.states.len(),
                comp.scripts.len()
            );
            Ok(())
        }
        "build" => {
            let target_path = if args.len() >= 3 { &args[2] } else { "." };
            let proj_dir = Path::new(target_path);

            println!("Building project for production via Rust engine...");
            let res = build_project(proj_dir, "dist")?;

            println!("\x1b[32m✔\x1b[0m Production build complete in \x1b[1m{}/\x1b[0m", res.output_dir);
            println!("  ├── index.html ({} bytes)", res.html_bytes);
            println!("  └── app.js ({} bytes)", res.js_bytes);
            if res.worker_bytes > 0 {
                println!("  ├── worker.py ({} bytes) [Python Server RPC]", res.worker_bytes);
            }
            if res.native_bytes > 0 {
                let lib_name = if cfg!(target_os = "windows") {
                    "native.dll"
                } else if cfg!(target_os = "macos") {
                    "libnative.dylib"
                } else {
                    "libnative.so"
                };
                println!("  └── {} ({} bytes) [Rust Native C-ABI cdylib]", lib_name, res.native_bytes);
            }
            Ok(())
        }
        "dev" => {
            let target_path = if args.len() >= 3 { &args[2] } else { "." };
            let proj_dir = Path::new(target_path);

            let toml_path = proj_dir.join("app.toml");
            let toml_source = std::fs::read_to_string(&toml_path)
                .map_err(|e| format!("failed to read app.toml: {}", e))?;
            let cfg = parse_app_toml(&toml_source)
                .map_err(|e| format!("invalid configuration in app.toml: {}", e))?;

            let mut server = DevServer::new(proj_dir, &cfg.dev.host, cfg.dev.port)?;
            server.serve(None)?;
            Ok(())
        }
        "clean" => {
            let target_path = if args.len() >= 3 { &args[2] } else { "." };
            let proj_dir = Path::new(target_path);

            println!("Cleaning build artifacts and cache...");
            let dist_dir = proj_dir.join("dist");
            if dist_dir.exists() {
                let _ = std::fs::remove_dir_all(&dist_dir);
            }
            let dev_dir = proj_dir.join(".zvelte");
            if dev_dir.exists() {
                let _ = std::fs::remove_dir_all(&dev_dir);
            }
            let rustby_cache = proj_dir.join(".rustby");
            if rustby_cache.exists() {
                let _ = std::fs::remove_dir_all(&rustby_cache);
            }

            println!("\x1b[32m✔\x1b[0m Clean complete. User source files preserved.");
            Ok(())
        }
        unknown => {
            Err(format!("unknown command '{}'. Run 'zvelte-rs --help' for available commands.", unknown))
        }
    }
}

pub fn print_help() {
    println!(
        r#"Rustby (Zvelte-RS) Compiler v{}
A Rust-powered, multi-language reactive web framework supporting Python server blocks
and Rust native C-ABI modules with fine-grained DOM reactivity.

USAGE:
  zvelte-rs <command> [options]

COMMANDS:
  new <name>       Create a new project with strict two-file layout (app.toml + app.zvelte)
  check [dir]      Type-check and validate component markup, state, and scripts
  build [dir]      Compile project to optimized production artifacts in dist/
  dev [dir]        Start local development server with automatic port probing & Python RPC
  clean [dir]      Remove build outputs and caches while strictly preserving user sources
  version, -v      Print version information
  help, -h         Print this help message
"#,
        VERSION
    );
}
