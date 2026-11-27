pub mod source;
pub mod config;
pub mod semantic;
pub mod compiler;
pub mod devserver;
pub mod cli;

pub use cli::run;

#[cfg(test)]
mod tests {
    use super::*;
    use source::parser::parse_component;
    use source::ast::*;
    use config::toml_parser::parse_app_toml;
    use semantic::analyzer::analyze;

    #[test]
    fn test_parse_and_analyze_python_and_rust_component() {
        let toml_src = r#"
[project]
name = "hybrid-rust-py"
version = "0.1.0"
entry = "app.zvelte"

[app]
target = "web"
mode = "hybrid"
renderer = "js"
strict = true
"#;
        let cfg = parse_app_toml(toml_src).expect("app.toml should parse");

        let zvelte_src = r#"
<script lang="rust">
#[no_mangle]
pub extern "C" fn rust_multiply(a: i32, b: i32) -> i32 {
    a * b
}
</script>

<script lang="python">
name = "Babar"
count = 10

def greeting(user):
    return f"Hello {user} from Python!"
</script>

<main>
    <h1>Hello {name}</h1>
    <p>{greeting(name)}</p>
    <button click={count++}>Count: {count}</button>
</main>
"#;
        let comp = parse_component(zvelte_src).expect("component should parse");
        assert_eq!(comp.scripts.len(), 2);
        assert_eq!(comp.scripts[0].lang, ScriptLang::Rust);
        assert_eq!(comp.scripts[1].lang, ScriptLang::Python);

        // Auto-extracted state variables
        assert!(comp.states.iter().any(|s| s.name == "name"));
        assert!(comp.states.iter().any(|s| s.name == "count"));

        let sem = analyze(comp, &cfg).expect("semantic analysis should pass");
        assert!(sem.func_names.contains("greeting"));
        assert!(sem.func_names.contains("rust_multiply"));
    }

    #[test]
    fn test_pure_javascript_svelte_style() {
        let src = r#"
<script>
let count = 0;
let title = "Svelte Style in Zvelte";
</script>

<h1>{title}</h1>
<button click={count++}>Count: {count}</button>
"#;
        let comp = parse_component(src).expect("component should parse");
        assert_eq!(comp.scripts.len(), 1);
        assert_eq!(comp.scripts[0].lang, ScriptLang::Js);
        assert_eq!(comp.scripts[0].context, ScriptContext::Client);
        assert_eq!(comp.states.len(), 2);
        assert_eq!(comp.states[0].name, "count");
        assert_eq!(comp.states[1].name, "title");
    }
}
