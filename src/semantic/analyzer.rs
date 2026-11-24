use std::collections::{HashMap, HashSet};
use crate::source::ast::*;
use crate::config::types::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticError {
    RustBlockNotAllowedInPythonMode,
    PythonBlockNotAllowedInRustMode,
    DuplicateStateVariable(String),
    UnsupportedStateType(String),
    UnknownStateInEvent(String),
    UnknownStateInBinding(String),
    UnknownCollectionInEach(String),
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticError::RustBlockNotAllowedInPythonMode => {
                write!(f, "Rust script block not allowed when app.mode is 'python'")
            }
            SemanticError::PythonBlockNotAllowedInRustMode => {
                write!(f, "Python script block not allowed when app.mode is 'rust'")
            }
            SemanticError::DuplicateStateVariable(name) => {
                write!(f, "duplicate state variable: '{}'", name)
            }
            SemanticError::UnsupportedStateType(t) => {
                write!(f, "unsupported state type: '{}'", t)
            }
            SemanticError::UnknownStateInEvent(id) => {
                write!(f, "unknown state variable or function in event handler: '{}'", id)
            }
            SemanticError::UnknownStateInBinding(id) => {
                write!(f, "unknown state variable in two-way binding: '{}'", id)
            }
            SemanticError::UnknownCollectionInEach(id) => {
                write!(f, "unknown collection variable in each block: '{}'", id)
            }
        }
    }
}

impl std::error::Error for SemanticError {}

#[derive(Debug, Clone)]
pub struct SemanticModel {
    pub component: Component,
    pub state_names: HashMap<String, StateVar>,
    pub func_names: HashSet<String>,
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "update"
            | "console"
            | "alert"
            | "fetch"
            | "window"
            | "document"
            | "setTimeout"
            | "setInterval"
            | "clearTimeout"
            | "clearInterval"
            | "true"
            | "false"
            | "null"
            | "undefined"
            | "this"
            | "event"
            | "Math"
            | "JSON"
            | "const"
            | "let"
            | "var"
            | "function"
            | "return"
            | "new"
            | "typeof"
            | "instanceof"
            | "void"
            | "delete"
            | "async"
            | "await"
            | "if"
            | "else"
            | "for"
            | "while"
            | "do"
            | "switch"
            | "case"
            | "break"
            | "continue"
            | "default"
            | "try"
            | "catch"
            | "finally"
            | "throw"
            | "in"
            | "of"
    )
}

fn extract_script_functions(script: &ScriptBlock, funcs: &mut HashSet<String>) {
    for raw_line in script.code.lines() {
        let line = raw_line.trim();
        if script.lang == ScriptLang::Python {
            let rest = if let Some(stripped) = line.strip_prefix("def ") {
                Some(stripped)
            } else if let Some(stripped) = line.strip_prefix("async def ") {
                Some(stripped)
            } else {
                None
            };
            if let Some(r) = rest {
                if let Some(paren_idx) = r.find('(') {
                    let fn_name = r[..paren_idx].trim();
                    if !fn_name.is_empty() {
                        funcs.insert(fn_name.to_string());
                    }
                }
            }
        } else if script.lang == ScriptLang::Rust {
            if let Some(fn_idx) = line.find("fn ") {
                let after_fn = &line[fn_idx + 3..];
                if let Some(paren_idx) = after_fn.find('(') {
                    let fn_name = after_fn[..paren_idx].trim();
                    if !fn_name.is_empty() {
                        funcs.insert(fn_name.to_string());
                    }
                }
            }
        } else if script.lang == ScriptLang::Js {
            let rest = if let Some(stripped) = line.strip_prefix("function ") {
                Some(stripped)
            } else if let Some(stripped) = line.strip_prefix("async function ") {
                Some(stripped)
            } else {
                None
            };
            if let Some(r) = rest {
                if let Some(paren_idx) = r.find('(') {
                    let fn_name = r[..paren_idx].trim();
                    if !fn_name.is_empty() {
                        funcs.insert(fn_name.to_string());
                    }
                }
            }
        }
    }
}

pub fn analyze(comp: Component, cfg: &AppConfig) -> Result<SemanticModel, SemanticError> {
    // 1. Validate script blocks against mode
    for script in &comp.scripts {
        match cfg.app.mode {
            AppMode::Python => {
                if script.lang != ScriptLang::Python && script.lang != ScriptLang::Js {
                    return Err(SemanticError::RustBlockNotAllowedInPythonMode);
                }
            }
            AppMode::Rust => {
                if script.lang != ScriptLang::Rust && script.lang != ScriptLang::Js {
                    return Err(SemanticError::PythonBlockNotAllowedInRustMode);
                }
            }
            AppMode::Hybrid => {}
        }
    }

    // 2. Build state symbol table
    let mut state_map = HashMap::new();
    for state in &comp.states {
        if state_map.contains_key(&state.name) {
            return Err(SemanticError::DuplicateStateVariable(state.name.clone()));
        }

        let valid_type = matches!(
            state.type_name.as_str(),
            "i32"
                | "i64"
                | "u32"
                | "u64"
                | "f32"
                | "f64"
                | "bool"
                | "string"
                | "any"
                | "list"
                | "dict"
                | "object"
                | "array"
        ) || state.type_name.starts_with("[]");

        if !valid_type {
            return Err(SemanticError::UnsupportedStateType(state.type_name.clone()));
        }

        state_map.insert(state.name.clone(), state.clone());
    }

    // 3. Extract declared functions from scripts
    let mut func_set = HashSet::new();
    for script in &comp.scripts {
        extract_script_functions(script, &mut func_set);
    }

    // 4. Validate event expressions and bindings in nodes
    let mut scope_vars = HashSet::new();
    validate_nodes(&comp.root_nodes, &state_map, &func_set, &mut scope_vars)?;

    Ok(SemanticModel {
        component: comp,
        state_names: state_map,
        func_names: func_set,
    })
}

fn validate_nodes(
    nodes: &[Node],
    states: &HashMap<String, StateVar>,
    funcs: &HashSet<String>,
    scope_vars: &mut HashSet<String>,
) -> Result<(), SemanticError> {
    for node in nodes {
        // Validate bindings
        for binding in &node.bindings {
            if !states.contains_key(&binding.variable) && !scope_vars.contains(&binding.variable) {
                return Err(SemanticError::UnknownStateInBinding(binding.variable.clone()));
            }
        }

        // Validate events
        for event in &node.events {
            validate_expr_identifiers(&event.expression, states, funcs, scope_vars)?;
        }

        // Validate if condition
        if let Some(cond) = &node.condition {
            validate_expr_identifiers(cond, states, funcs, scope_vars)?;
        }

        // Validate each block
        if let Some(col) = &node.collection {
            if !states.contains_key(col) && !scope_vars.contains(col) {
                return Err(SemanticError::UnknownCollectionInEach(col.clone()));
            }
            if let Some(item) = &node.item_name {
                scope_vars.insert(item.clone());
            }
        }

        // Recurse children
        validate_nodes(&node.children, states, funcs, scope_vars)?;
        validate_nodes(&node.consequent, states, funcs, scope_vars)?;
        validate_nodes(&node.alternate, states, funcs, scope_vars)?;

        if let Some(item) = &node.item_name {
            scope_vars.remove(item);
        }
    }
    Ok(())
}

fn validate_expr_identifiers(
    expr: &str,
    states: &HashMap<String, StateVar>,
    funcs: &HashSet<String>,
    scope_vars: &HashSet<String>,
) -> Result<(), SemanticError> {
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    let mut local_vars: HashSet<String> = HashSet::new();
    let mut prev_ident: Option<String> = None;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let is_prop_access = start > 0 && chars[start - 1] == '.';

            // Track declarations: const x, let x, var x
            if let Some(ref p) = prev_ident {
                if p == "const" || p == "let" || p == "var" {
                    local_vars.insert(ident.clone());
                }
            }

            // Track arrow function parameter: ident =>
            let mut peek_idx = i;
            while peek_idx < chars.len() && chars[peek_idx].is_whitespace() {
                peek_idx += 1;
            }
            if peek_idx + 1 < chars.len() && chars[peek_idx] == '=' && chars[peek_idx + 1] == '>' {
                local_vars.insert(ident.clone());
            }

            if !is_prop_access
                && !is_builtin(&ident)
                && !local_vars.contains(&ident)
                && !states.contains_key(&ident)
                && !funcs.contains(&ident)
                && !scope_vars.contains(&ident)
            {
                if !ident.chars().all(|ch| ch.is_ascii_digit()) {
                    return Err(SemanticError::UnknownStateInEvent(ident));
                }
            }
            prev_ident = Some(ident);
        } else {
            if !c.is_whitespace() {
                if c == ';' || c == '{' || c == '}' || c == '(' || c == ')' || c == ',' {
                    prev_ident = None;
                }
            }
            i += 1;
        }
    }

    Ok(())
}
