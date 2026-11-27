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
    UnknownStateInExpression(String),
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
            SemanticError::UnknownStateInExpression(id) => {
                write!(f, "unknown state variable or function in template expression: '{}'", id)
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
            | "Array"
            | "Object"
            | "String"
            | "Number"
            | "Boolean"
            | "Date"
            | "Promise"
            | "Error"
            | "Map"
            | "Set"
            | "Intl"
            | "URL"
            | "parseInt"
            | "parseFloat"
            | "isNaN"
            | "isFinite"
            | "encodeURIComponent"
            | "decodeURIComponent"
            | "Infinity"
            | "NaN"
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
                Some((stripped, false))
            } else if let Some(stripped) = line.strip_prefix("async function ") {
                Some((stripped, false))
            } else if let Some(stripped) = line.strip_prefix("const ") {
                Some((stripped, true))
            } else if let Some(stripped) = line.strip_prefix("let ") {
                Some((stripped, true))
            } else {
                None
            };
            if let Some((r, is_const_arrow)) = rest {
                if is_const_arrow {
                    if let Some(eq_idx) = r.find('=') {
                        let rhs = r[eq_idx + 1..].trim();
                        if rhs.starts_with("function")
                            || rhs.starts_with("async ")
                            || rhs.contains("=>")
                        {
                            let fn_name = r[..eq_idx].trim();
                            if !fn_name.is_empty() {
                                funcs.insert(fn_name.to_string());
                            }
                        }
                    }
                } else if let Some(paren_idx) = r.find('(') {
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

    // 3. Extract declared functions & top-level JS client helpers from scripts
    let mut func_set = HashSet::new();
    for script in &comp.scripts {
        extract_script_functions(script, &mut func_set);
    }

    // Also include `$:` reactive derived variables as readable symbols and validate their expressions
    let mut scope_vars = HashSet::new();
    for der in &comp.derived {
        validate_expr_identifiers(&der.expression, &state_map, &func_set, &scope_vars, true)?;
        state_map.entry(der.name.clone()).or_insert(StateVar {
            name: der.name.clone(),
            type_name: "any".to_string(),
            initial_value: "null".to_string(),
            line: der.line,
        });
    }

    // 4. Validate events, bindings, conditions, each blocks, and template expressions
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
        // Validate dynamic attribute `{expr}` bindings
        for attr in &node.attributes {
            if attr.value.starts_with('{') && attr.value.ends_with('}') && attr.value.len() >= 2 {
                let expr = &attr.value[1..attr.value.len() - 1];
                validate_expr_identifiers(expr, states, funcs, scope_vars, false)?;
            }
        }

        // Validate `{expr}` dynamic text interpolations
        if node.node_type == NodeType::DynamicText {
            validate_expr_identifiers(&node.content, states, funcs, scope_vars, false)?;
        }

        // Validate bindings
        for binding in &node.bindings {
            if !states.contains_key(&binding.variable) && !scope_vars.contains(&binding.variable) {
                return Err(SemanticError::UnknownStateInBinding(binding.variable.clone()));
            }
        }

        // Validate events
        for event in &node.events {
            validate_expr_identifiers(&event.expression, states, funcs, scope_vars, true)?;
        }

        // Validate if condition
        if let Some(cond) = &node.condition {
            validate_expr_identifiers(cond, states, funcs, scope_vars, false)?;
        }

        // Validate each block (preserve any shadowed outer scope vars cleanly)
        let mut pushed_locals: Vec<(String, bool)> = Vec::new();
        if let Some(col) = &node.collection {
            if !states.contains_key(col) && !scope_vars.contains(col) {
                return Err(SemanticError::UnknownCollectionInEach(col.clone()));
            }
            if let Some(item) = &node.item_name {
                let was_present = scope_vars.contains(item);
                scope_vars.insert(item.clone());
                pushed_locals.push((item.clone(), was_present));
            }
            if let Some(idx) = &node.index_name {
                let was_present = scope_vars.contains(idx);
                scope_vars.insert(idx.clone());
                pushed_locals.push((idx.clone(), was_present));
            }
        }

        // Recurse children
        validate_nodes(&node.children, states, funcs, scope_vars)?;
        validate_nodes(&node.consequent, states, funcs, scope_vars)?;
        validate_nodes(&node.alternate, states, funcs, scope_vars)?;

        for (local_name, was_present) in pushed_locals {
            if !was_present {
                scope_vars.remove(&local_name);
            }
        }
    }
    Ok(())
}

fn validate_expr_identifiers(
    expr: &str,
    states: &HashMap<String, StateVar>,
    funcs: &HashSet<String>,
    scope_vars: &HashSet<String>,
    is_event: bool,
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

        // Handle template literal backticks: skip static text and recursively check `${...}`
        if c == '`' {
            i += 1;
            while i < chars.len() {
                let sc = chars[i];
                if sc == '\\' && i + 1 < chars.len() {
                    i += 2;
                    continue;
                }
                if sc == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                    i += 2;
                    let mut inner = String::new();
                    let mut depth = 1usize;
                    let mut in_q: Option<char> = None;
                    let mut esc = false;
                    while i < chars.len() {
                        let ic = chars[i];
                        if let Some(iq) = in_q {
                            if esc {
                                esc = false;
                            } else if ic == '\\' {
                                esc = true;
                            } else if ic == iq {
                                in_q = None;
                            }
                        } else if ic == '"' || ic == '\'' || ic == '`' {
                            in_q = Some(ic);
                        } else if ic == '{' {
                            depth += 1;
                        } else if ic == '}' {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        inner.push(ic);
                        i += 1;
                    }
                    let mut combined_scope = scope_vars.clone();
                    combined_scope.extend(local_vars.iter().cloned());
                    validate_expr_identifiers(&inner, states, funcs, &combined_scope, is_event)?;
                    continue;
                }
                i += 1;
                if sc == '`' {
                    break;
                }
            }
            continue;
        }

        if c == '(' {
            // Scan matching ')' to detect `(a, b) =>` multi-parameter arrow closures
            let mut p = i + 1;
            let mut depth = 1usize;
            while p < chars.len() && depth > 0 {
                match chars[p] {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                p += 1;
            }
            if depth == 0 {
                let mut after = p;
                while after < chars.len() && chars[after].is_whitespace() {
                    after += 1;
                }
                if after + 1 < chars.len() && chars[after] == '=' && chars[after + 1] == '>' {
                    let inner: String = chars[i + 1..p - 1].iter().collect();
                    for param in inner.split(',') {
                        let name = param.split('=').next().unwrap_or("").trim();
                        if !name.is_empty()
                            && name
                                .chars()
                                .all(|ch| ch.is_alphanumeric() || ch == '_')
                        {
                            local_vars.insert(name.to_string());
                        }
                    }
                }
            }
            prev_ident = None;
            i += 1;
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

            // Track arrow function parameter: ident => or (a, b) =>
            let mut peek_idx = i;
            while peek_idx < chars.len() && chars[peek_idx].is_whitespace() {
                peek_idx += 1;
            }
            if peek_idx + 1 < chars.len() && chars[peek_idx] == '=' && chars[peek_idx + 1] == '>' {
                local_vars.insert(ident.clone());
            }

            // Skip JS object literal keys (`{ key: val, other: val }`)
            let prev_sig = (0..start)
                .rev()
                .map(|idx| chars[idx])
                .find(|ch| !ch.is_whitespace());
            let is_object_key = peek_idx < chars.len()
                && chars[peek_idx] == ':'
                && matches!(prev_sig, Some('{') | Some(','));

            if !is_prop_access
                && !is_object_key
                && !is_builtin(&ident)
                && !local_vars.contains(&ident)
                && !states.contains_key(&ident)
                && !funcs.contains(&ident)
                && !scope_vars.contains(&ident)
            {
                if !ident.chars().all(|ch| ch.is_ascii_digit()) {
                    return Err(if is_event {
                        SemanticError::UnknownStateInEvent(ident)
                    } else {
                        SemanticError::UnknownStateInExpression(ident)
                    });
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
