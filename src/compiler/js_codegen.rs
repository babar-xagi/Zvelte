use std::collections::HashSet;
use crate::source::ast::*;

#[derive(Debug, Clone)]
pub struct CodegenOutput {
    pub html: String,
    pub js: String,
}

pub struct JsCodegen<'a> {
    component: &'a Component,
    project_name: &'a str,
    var_id: usize,
    _in_dynamic_block: bool,
    expand_variant_groups: bool,
    local_vars: Vec<String>,
}

impl<'a> JsCodegen<'a> {
    pub fn new(component: &'a Component, project_name: &'a str) -> Self {
        Self {
            component,
            project_name,
            var_id: 0,
            _in_dynamic_block: false,
            expand_variant_groups: true,
            local_vars: Vec::new(),
        }
    }

    fn is_active_state_var(&self, ident: &str, extra_locals: &HashSet<String>) -> bool {
        if extra_locals.contains(ident) || self.local_vars.iter().any(|v| v == ident) {
            return false;
        }
        self.component.states.iter().any(|s| s.name == ident)
            || self.component.derived.iter().any(|d| d.name == ident)
    }

    fn next_var(&mut self, prefix: &str) -> String {
        let id = self.var_id;
        self.var_id += 1;
        format!("{}_{}", prefix, id)
    }

    fn escape_js_string(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 8);
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(c),
            }
        }
        out
    }

    fn escape_html(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                _ => out.push(c),
            }
        }
        out
    }

    pub fn format_expr(&self, expr: &str) -> String {
        let empty = HashSet::new();
        self.format_expr_with_locals(expr, &empty)
    }

    fn format_expr_with_locals(&self, expr: &str, extra_locals: &HashSet<String>) -> String {
        let mut out = String::new();
        let chars: Vec<char> = expr.chars().collect();
        let mut i = 0;
        let mut scoped_locals = extra_locals.clone();
        let mut prev_ident: Option<String> = None;

        while i < chars.len() {
            let c = chars[i];
            if c == '"' || c == '\'' || c == '`' {
                let quote = c;
                out.push(quote);
                i += 1;
                while i < chars.len() {
                    let sc = chars[i];
                    if sc == '\\' && i + 1 < chars.len() {
                        out.push(sc);
                        i += 1;
                        out.push(chars[i]);
                        i += 1;
                        continue;
                    }
                    // Inside backtick template literals, format ${...} expressions recursively
                    if quote == '`' && sc == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                        out.push_str("${");
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
                        out.push_str(&self.format_expr_with_locals(&inner, &scoped_locals));
                        out.push('}');
                        continue;
                    }
                    out.push(sc);
                    i += 1;
                    if sc == quote {
                        break;
                    }
                }
                continue;
            }
            if c == '(' {
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
                                scoped_locals.insert(name.to_string());
                            }
                        }
                    }
                }
                prev_ident = None;
                out.push(c);
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

                if let Some(ref prev) = prev_ident {
                    if prev == "let" || prev == "const" || prev == "var" || prev == "function" {
                        scoped_locals.insert(ident.clone());
                    }
                }

                let mut peek_idx = i;
                while peek_idx < chars.len() && chars[peek_idx].is_whitespace() {
                    peek_idx += 1;
                }
                if peek_idx + 1 < chars.len() && chars[peek_idx] == '=' && chars[peek_idx + 1] == '>' {
                    scoped_locals.insert(ident.clone());
                }

                let prev_sig = (0..start)
                    .rev()
                    .map(|idx| chars[idx])
                    .find(|ch| !ch.is_whitespace());
                let is_object_key = peek_idx < chars.len()
                    && chars[peek_idx] == ':'
                    && matches!(prev_sig, Some('{') | Some(','));

                let is_state = !is_prop_access
                    && !is_object_key
                    && self.is_active_state_var(&ident, &scoped_locals);
                if is_state {
                    out.push_str("state.");
                    out.push_str(&ident);
                } else {
                    out.push_str(&ident);
                }
                prev_ident = Some(ident);
            } else {
                if !c.is_whitespace() && matches!(c, ';' | '{' | '}' | '(' | ')' | ',') {
                    prev_ident = None;
                }
                out.push(c);
                i += 1;
            }
        }

        out
    }

    pub fn extract_js_functions(code: &str) -> Vec<(String, bool)> {
        let mut funcs = Vec::new();
        for raw_line in code.lines() {
            let line = raw_line.trim();
            if let Some(r) = line
                .strip_prefix("function ")
                .or_else(|| line.strip_prefix("async function "))
            {
                if let Some((lp, rp)) = r.find('(').and_then(|lp| r.find(')').map(|rp| (lp, rp))) {
                    let fn_name = r[..lp].trim();
                    let params = r[lp + 1..rp].trim();
                    let pass_event = !params.is_empty()
                        && !params.split(',').next().unwrap_or("").contains('=');
                    if !fn_name.is_empty() {
                        funcs.push((fn_name.to_string(), pass_event));
                    }
                }
            } else if let Some(r) = line
                .strip_prefix("const ")
                .or_else(|| line.strip_prefix("let "))
            {
                if let Some(eq_idx) = r.find('=') {
                    let rhs = r[eq_idx + 1..].trim();
                    if rhs.starts_with("function")
                        || rhs.starts_with("async ")
                        || rhs.contains("=>")
                    {
                        let fn_name = r[..eq_idx].trim();
                        let pass_event = rhs
                            .find('(')
                            .and_then(|lp| rhs.find(')').map(|rp| &rhs[lp + 1..rp]))
                            .map(|p| {
                                let p = p.trim();
                                !p.is_empty() && !p.split(',').next().unwrap_or("").contains('=')
                            })
                            .unwrap_or(false);
                        if !fn_name.is_empty() {
                            funcs.push((fn_name.to_string(), pass_event));
                        }
                    }
                }
            }
        }
        funcs
    }

    fn rewrite_rpc_calls_with_await(expr: &str, rpc_funcs: &HashSet<String>) -> (String, bool) {
        if rpc_funcs.is_empty() {
            let has_await = expr.contains("await ");
            return (expr.to_string(), has_await);
        }

        let chars: Vec<char> = expr.chars().collect();
        let mut out = String::with_capacity(expr.len() + 16);
        let mut i = 0;
        let mut used_async = false;

        while i < chars.len() {
            let c = chars[i];
            if c == '"' || c == '\'' || c == '`' {
                let q = c;
                out.push(q);
                i += 1;
                while i < chars.len() {
                    let sc = chars[i];
                    if sc == '\\' && i + 1 < chars.len() {
                        out.push(sc);
                        out.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    out.push(sc);
                    i += 1;
                    if sc == q {
                        break;
                    }
                }
                continue;
            }

            if c.is_alphabetic() || c == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                if ident == "await" {
                    used_async = true;
                }

                let is_prop = start > 0 && chars[start - 1] == '.';
                let mut peek = i;
                while peek < chars.len() && chars[peek].is_whitespace() {
                    peek += 1;
                }

                if !is_prop && rpc_funcs.contains(&ident) && peek < chars.len() && chars[peek] == '(' {
                    // Find matching closing paren
                    let mut p = peek + 1;
                    let mut depth = 1usize;
                    let mut in_q: Option<char> = None;
                    let mut esc = false;
                    while p < chars.len() && depth > 0 {
                        let pc = chars[p];
                        if let Some(iq) = in_q {
                            if esc {
                                esc = false;
                            } else if pc == '\\' {
                                esc = true;
                            } else if pc == iq {
                                in_q = None;
                            }
                        } else if pc == '"' || pc == '\'' || pc == '`' {
                            in_q = Some(pc);
                        } else if pc == '(' {
                            depth += 1;
                        } else if pc == ')' {
                            depth -= 1;
                        }
                        p += 1;
                    }

                    let mut after = p;
                    while after < chars.len() && chars[after].is_whitespace() {
                        after += 1;
                    }
                    let rest_after: String = chars[after..].iter().collect();
                    let chained_promise = rest_after.starts_with(".then(")
                        || rest_after.starts_with(".catch(")
                        || rest_after.starts_with(".finally(");

                    let trimmed_out = out.trim_end();
                    let already_awaited = trimmed_out.ends_with("await");
                    if !chained_promise && !already_awaited {
                        out.push_str("await ");
                        used_async = true;
                    }
                }

                out.push_str(&ident);
            } else {
                out.push(c);
                i += 1;
            }
        }

        (out, used_async)
    }

    fn count_braces_outside_quotes(line: &str) -> i32 {
        let mut delta = 0i32;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' || c == '\'' || c == '`' {
                let q = c;
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                        continue;
                    }
                    if chars[i] == q {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            if c == '{' {
                delta += 1;
            } else if c == '}' {
                delta -= 1;
            }
            i += 1;
        }
        delta
    }

    fn compile_client_script(&self, code: &str) -> String {
        let mut out = String::new();
        let mut brace_depth = 0i32;
        let mut fn_scopes: Vec<(i32, HashSet<String>, bool)> = Vec::new();

        for raw_line in code.lines() {
            let trimmed = raw_line.trim();
            if brace_depth == 0 {
                if trimmed.starts_with("$:") {
                    continue;
                }
                // Skip top-level hoisted state variable declarations (`let count = 0;`)
                let stripped_decl = trimmed
                    .strip_prefix("let ")
                    .or_else(|| trimmed.strip_prefix("var "))
                    .or_else(|| trimmed.strip_prefix("const "))
                    .map(|s| s.trim());
                if let Some(decl_body) = stripped_decl {
                    if let Some(eq_idx) = decl_body.find('=') {
                        let lhs = decl_body[..eq_idx].trim();
                        if self.component.states.iter().any(|s| s.name == lhs) {
                            let is_fn_expr = decl_body[eq_idx + 1..].trim_start().starts_with("function")
                                || decl_body[eq_idx + 1..].contains("=>");
                            if !is_fn_expr && Self::count_braces_outside_quotes(trimmed) == 0 {
                                continue;
                            }
                        }
                    }
                }
            }

            // Detect function entry (`function f(a)` or `const f = (a) => {`) to track parameter locals and state mutations
            let fn_params: Option<HashSet<String>> = if let Some(after_kw) = trimmed
                .strip_prefix("async function ")
                .or_else(|| trimmed.strip_prefix("function "))
            {
                if let (Some(lp), Some(rp)) = (after_kw.find('('), after_kw.find(')')) {
                    Some(
                        after_kw[lp + 1..rp]
                            .split(',')
                            .map(|p| p.split('=').next().unwrap_or("").trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect(),
                    )
                } else {
                    None
                }
            } else if trimmed
                .strip_prefix("const ")
                .or_else(|| trimmed.strip_prefix("let "))
                .is_some_and(|r| r.contains("=>") && trimmed.ends_with('{'))
            {
                if let (Some(lp), Some(rp)) = (trimmed.find('('), trimmed.find(')')) {
                    Some(
                        trimmed[lp + 1..rp]
                            .split(',')
                            .map(|p| p.split('=').next().unwrap_or("").trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect(),
                    )
                } else {
                    Some(HashSet::new())
                }
            } else {
                None
            };

            if let Some(params) = fn_params {
                let next_depth = brace_depth + Self::count_braces_outside_quotes(raw_line);
                if next_depth > brace_depth {
                    fn_scopes.push((brace_depth, params, false));
                    out.push_str(raw_line);
                    out.push('\n');
                    brace_depth = next_depth;
                    continue;
                }
            }

            let mut active_locals = HashSet::new();
            for (_, params, _) in &fn_scopes {
                active_locals.extend(params.iter().cloned());
            }

            // Track local declarations inside function bodies cleanly
            if brace_depth > 0 {
                if let Some(decl) = trimmed
                    .strip_prefix("let ")
                    .or_else(|| trimmed.strip_prefix("const "))
                    .or_else(|| trimmed.strip_prefix("var "))
                {
                    let name: String = decl
                        .trim()
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        if let Some((_, params, _)) = fn_scopes.last_mut() {
                            params.insert(name);
                        }
                    }
                }
            }

            let delta = Self::count_braces_outside_quotes(raw_line);
            let closing_fn = delta < 0
                && fn_scopes
                    .last()
                    .is_some_and(|(entry_d, _, _)| brace_depth + delta <= *entry_d);

            if closing_fn && trimmed == "}" {
                if let Some((_, _, mutated)) = fn_scopes.pop() {
                    if mutated {
                        out.push_str("    if (typeof update === \"function\") update();\n");
                    }
                }
                out.push_str(raw_line);
                out.push('\n');
                brace_depth += delta;
                continue;
            }

            let rewritten = self.format_expr_with_locals(raw_line, &active_locals);
            if let Some((_, _, mutated)) = fn_scopes.last_mut() {
                if rewritten.contains("state.") {
                    *mutated = true;
                }
            }
            out.push_str(&rewritten);
            out.push('\n');
            brace_depth += delta;

            if closing_fn {
                fn_scopes.pop();
            }
        }

        out.trim().to_string()
    }

    pub fn minify_output(out: CodegenOutput) -> CodegenOutput {
        CodegenOutput {
            html: Self::minify_html_and_css(&out.html),
            js: Self::minify_js(&out.js),
        }
    }

    fn minify_html_and_css(html: &str) -> String {
        html.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn minify_js(js: &str) -> String {
        let mut lines = Vec::new();
        for raw_line in js.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            lines.push(trimmed);
        }
        lines.join("\n")
    }
    pub fn extract_python_function_names(code: &str) -> Vec<String> {
        let mut names = Vec::new();
        for raw_line in code.lines() {
            let line = raw_line.trim();
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
                        names.push(fn_name.to_string());
                    }
                }
            }
        }
        names
    }

    pub fn extract_rust_function_names(code: &str) -> Vec<String> {
        let mut names = Vec::new();
        for raw_line in code.lines() {
            let line = raw_line.trim();
            if line.contains("extern \"C\" fn ") || line.contains("extern fn ") {
                if let Some(fn_idx) = line.find("fn ") {
                    let after_fn = &line[fn_idx + 3..];
                    if let Some(paren_idx) = after_fn.find('(') {
                        let fn_name = after_fn[..paren_idx].trim();
                        if !fn_name.is_empty() {
                            names.push(fn_name.to_string());
                        }
                    }
                }
            }
        }
        names
    }

    pub fn generate_html(&self) -> String {
        self.generate_html_with_css(None, &self.component.styles, None)
    }

    pub fn generate_html_with_css(
        &self,
        tailwind_base: Option<&str>,
        custom_styles: &str,
        tailwind_utilities: Option<&str>,
    ) -> String {
        let mut combined_css = String::new();
        if let Some(base) = tailwind_base {
            if !base.trim().is_empty() {
                combined_css.push_str(base.trim());
                combined_css.push('\n');
            }
        }
        if !custom_styles.trim().is_empty() {
            combined_css.push_str(custom_styles.trim());
            combined_css.push('\n');
        }
        if let Some(utils) = tailwind_utilities {
            if !utils.trim().is_empty() {
                combined_css.push_str(utils.trim());
                combined_css.push('\n');
            }
        }

        let style_block = if !combined_css.trim().is_empty() {
            format!("  <style>\n{}\n  </style>\n", combined_css.trim_end())
        } else {
            String::new()
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{} - Zvelte</title>
{}</head>
<body>
  <div id="app"></div>
  <script src="app.js"></script>
</body>
</html>
"#,
            Self::escape_html(self.project_name), style_block
        )
    }

    pub fn generate_js(&mut self) -> String {
        let mut sb = String::new();

        sb.push_str(
            r#"(function() {
  "use strict";

  // Zvelte Reactive Runtime (Fine-Grained DOM Updates)
  const state = {
"#,
        );

        // Emit state initializers + `$:` derived slots
        for state in &self.component.states {
            sb.push_str(&format!("    {}: {},\n", state.name, state.initial_value));
        }
        for der in &self.component.derived {
            if !self.component.states.iter().any(|s| s.name == der.name) {
                sb.push_str(&format!("    {}: null,\n", der.name));
            }
        }

        sb.push_str(
            r#"  };

  function __recompute_derived() {
"#,
        );
        for der in &self.component.derived {
            let formatted = self.format_expr(&der.expression);
            sb.push_str(&format!("    state.{} = {};\n", der.name, formatted));
        }
        sb.push_str(
            r#"  }
  __recompute_derived();

  let subscribers = [];
  let __curr_scope = "root";
  let __scope_seq = 0;
  let __updating = false;
  function subscribe(fn) {
    subscribers.push({ fn: fn, scope: __curr_scope });
  }
  function __clear_scope(prefix) {
    subscribers = subscribers.filter(function(s) {
      return s.scope !== prefix && !s.scope.startsWith(prefix + "/");
    });
  }
  function update() {
    if (__updating) return;
    __updating = true;
    __recompute_derived();
    try {
      const snapshot = subscribers.slice();
      for (let i = 0; i < snapshot.length; i++) {
        snapshot[i].fn();
      }
    } finally {
      __updating = false;
    }
  }

"#,
        );

        // Check for server/native RPC scripts
        let has_server_rpc = self.component.scripts.iter().any(|s| {
            (s.lang == ScriptLang::Python && s.context == ScriptContext::Server)
                || (s.lang == ScriptLang::Rust && s.context == ScriptContext::Native)
        });

        if has_server_rpc {
            sb.push_str(
                r#"  // Zvelte Server & Native RPC Bridge
  async function __zvelte_rpc(method, params) {
    try {
      const response = await fetch("/api/rpc", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ method: method, params: params || [] })
      });
      const data = await response.json();
      if (data && data.error) {
        console.error("[Zvelte RPC Error]", data.error);
        return "RPC Error: " + data.error;
      }
      return data ? data.result : "No response data";
    } catch (err) {
      console.error("[Zvelte RPC Network Error]", err);
      return "Network Error: " + err;
    }
  }
  window.__zvelte_rpc = __zvelte_rpc;

"#,
            );

            for script in &self.component.scripts {
                if script.lang == ScriptLang::Python && script.context == ScriptContext::Server {
                    let fn_names = Self::extract_python_function_names(&script.code);
                    for fn_name in fn_names {
                        sb.push_str(&format!(
                            r#"  async function {}(...args) {{
    return await __zvelte_rpc("{}", args);
  }}
  window.{} = {};

"#,
                            fn_name, fn_name, fn_name, fn_name
                        ));
                    }
                } else if script.lang == ScriptLang::Rust && script.context == ScriptContext::Native {
                    let fn_names = Self::extract_rust_function_names(&script.code);
                    for fn_name in fn_names {
                        sb.push_str(&format!(
                            r#"  async function {}(...args) {{
    return await __zvelte_rpc("{}", args);
  }}
  window.{} = {};

"#,
                            fn_name, fn_name, fn_name, fn_name
                        ));
                    }
                }
            }
        }

        // Inject user client-side JavaScript scripts (compiled with Svelte-style reactivity)
        for script in &self.component.scripts {
            if script.lang == ScriptLang::Js && script.context == ScriptContext::Client {
                let compiled_client = self.compile_client_script(&script.code);
                if !compiled_client.is_empty() {
                    sb.push_str("\n  // User Client Script\n  ");
                    sb.push_str(&compiled_client.replace('\n', "\n  "));
                    sb.push_str("\n");
                }
            }
        }

        sb.push_str(
            r#"  document.addEventListener("DOMContentLoaded", function() {
    const root = document.getElementById("app");
    if (!root) return;

    function __setNodeText(node, val) {
      if (val && typeof val.then === "function") {
        val.then(res => { node.data = res != null ? res : ""; });
      } else {
        node.data = val != null ? val : "";
      }
    }

"#,
        );

        let root_nodes = self.component.root_nodes.clone();
        for node in &root_nodes {
            let node_var = self.emit_node(&mut sb, node);
            sb.push_str(&format!("    root.appendChild({});\n", node_var));
        }

        sb.push_str(
            r#"  });
})();
"#,
        );

        sb
    }

    fn emit_node(&mut self, sb: &mut String, node: &Node) -> String {
        match node.node_type {
            NodeType::Element => {
                let el_var = self.next_var("el");
                sb.push_str(&format!(
                    "    const {} = document.createElement(\"{}\");\n",
                    el_var, node.tag
                ));

                // Attributes
                for attr in &node.attributes {
                    let expand_attr = self.expand_variant_groups
                        && (attr.name == "class"
                            || crate::compiler::unocss::is_grouped_attributify_attr(
                                &node.tag,
                                &attr.name,
                            ));
                    let attr_name_esc = Self::escape_js_string(&attr.name);

                    if attr.value.starts_with('{') && attr.value.ends_with('}') && attr.value.len() >= 2 {
                        let expr = &attr.value[1..attr.value.len() - 1];
                        let normalized_expr = if expand_attr {
                            crate::compiler::unocss::expand_variant_groups_in_expr(expr)
                        } else {
                            expr.to_string()
                        };
                        let formatted = self.format_expr(&normalized_expr);
                        sb.push_str(&format!(
                            "    {}.setAttribute(\"{}\", {});\n",
                            el_var, attr_name_esc, formatted
                        ));
                        sb.push_str(&format!(
                            "    subscribe(() => {{ {}.setAttribute(\"{}\", {}); }});\n",
                            el_var, attr_name_esc, formatted
                        ));
                    } else {
                        let val = if expand_attr && !attr.value.is_empty() {
                            crate::compiler::unocss::expand_variant_groups(&attr.value)
                        } else {
                            attr.value.clone()
                        };
                        sb.push_str(&format!(
                            "    {}.setAttribute(\"{}\", \"{}\");\n",
                            el_var,
                            attr_name_esc,
                            Self::escape_js_string(&val)
                        ));
                    }
                }

                // Two-way bindings (bind:value / bind:checked)
                for binding in &node.bindings {
                    let var_name = &binding.variable;
                    let is_state = !self.local_vars.iter().any(|v| v == var_name)
                        && self.component.states.iter().any(|s| s.name == *var_name);
                    let target_expr = if is_state {
                        format!("state.{}", var_name)
                    } else {
                        var_name.clone()
                    };

                    if binding.property == "value" {
                        sb.push_str(&format!("    {}.value = {};\n", el_var, target_expr));
                        sb.push_str(&format!(
                            "    subscribe(() => {{ if ({}.value !== {}) {}.value = {} != null ? {} : \"\"; }});\n",
                            el_var, target_expr, el_var, target_expr, target_expr
                        ));
                        sb.push_str(&format!(
                            "    {}.addEventListener(\"input\", function(e) {{ {} = e.target.value; update(); }});\n",
                            el_var, target_expr
                        ));
                    } else if binding.property == "checked" {
                        sb.push_str(&format!("    {}.checked = Boolean({});\n", el_var, target_expr));
                        sb.push_str(&format!(
                            "    subscribe(() => {{ {}.checked = Boolean({}); }});\n",
                            el_var, target_expr
                        ));
                        sb.push_str(&format!(
                            "    {}.addEventListener(\"change\", function(e) {{ {} = e.target.checked; update(); }});\n",
                            el_var, target_expr
                        ));
                    }
                }

                // Collect RPC & client JS functions once per element for smart handler wrapping
                let mut rpc_funcs = HashSet::new();
                let mut client_funcs = std::collections::HashMap::new();
                for script in &self.component.scripts {
                    if script.lang == ScriptLang::Python && script.context == ScriptContext::Server {
                        rpc_funcs.extend(Self::extract_python_function_names(&script.code));
                    } else if script.lang == ScriptLang::Rust && script.context == ScriptContext::Native {
                        rpc_funcs.extend(Self::extract_rust_function_names(&script.code));
                    } else if script.lang == ScriptLang::Js && script.context == ScriptContext::Client {
                        for (name, pass_event) in Self::extract_js_functions(&script.code) {
                            client_funcs.insert(name, pass_event);
                        }
                    }
                }

                // Events (click / on:click / onclick / modifiers / arrow callbacks / awaited RPC)
                for event in &node.events {
                    let raw_trim = event.expression.trim();
                    let mut mod_prefix = String::new();
                    let mut is_once = false;
                    for m in &event.modifiers {
                        match m.as_str() {
                            "preventDefault" => mod_prefix.push_str("event.preventDefault(); "),
                            "stopPropagation" => mod_prefix.push_str("event.stopPropagation(); "),
                            "once" => is_once = true,
                            _ => {}
                        }
                    }

                    let formatted_base = if rpc_funcs.contains(raw_trim) {
                        format!("await {}()", raw_trim)
                    } else if let Some(&pass_event) = client_funcs.get(raw_trim) {
                        if pass_event {
                            format!("{}(event)", raw_trim)
                        } else {
                            format!("{}()", raw_trim)
                        }
                    } else {
                        let f = self.format_expr(&event.expression);
                        let is_arrow = raw_trim.starts_with('(') && raw_trim.contains("=>")
                            || raw_trim.split_whitespace().next().is_some_and(|w| raw_trim[w.len()..].trim_start().starts_with("=>"));
                        if is_arrow {
                            let (awaited_arrow, _) = Self::rewrite_rpc_calls_with_await(&f, &rpc_funcs);
                            if awaited_arrow.contains("await ") && !awaited_arrow.trim_start().starts_with("async ") {
                                format!("await (async {})(event)", awaited_arrow)
                            } else if awaited_arrow.contains("await ") {
                                format!("await ({})(event)", awaited_arrow)
                            } else {
                                format!("({})(event)", awaited_arrow)
                            }
                        } else {
                            let (awaited_stmt, _) = Self::rewrite_rpc_calls_with_await(&f, &rpc_funcs);
                            awaited_stmt
                        }
                    };

                    let needs_async = formatted_base.contains("await ");
                    let async_kw = if needs_async { "async " } else { "" };
                    let opts_arg = if is_once { ", { once: true }" } else { "" };
                    sb.push_str(&format!(
                        "    {}.addEventListener(\"{}\", {}function(event) {{ {}{}; update(); }}{});\n",
                        el_var, event.event_name, async_kw, mod_prefix, formatted_base, opts_arg
                    ));
                }

                // Children
                for child in &node.children {
                    let child_var = self.emit_node(sb, child);
                    sb.push_str(&format!("    {}.appendChild({});\n", el_var, child_var));
                }

                el_var
            }
            NodeType::Text => {
                let txt_var = self.next_var("txt");
                let escaped = Self::escape_js_string(&node.content);
                sb.push_str(&format!(
                    "    const {} = document.createTextNode(\"{}\");\n",
                    txt_var, escaped
                ));
                txt_var
            }
            NodeType::DynamicText => {
                let dyn_var = self.next_var("txt_dyn");
                sb.push_str(&format!(
                    "    const {} = document.createTextNode(\"\");\n",
                    dyn_var
                ));
                let formatted = self.format_expr(&node.content);
                sb.push_str(&format!("    __setNodeText({}, {});\n", dyn_var, formatted));
                sb.push_str(&format!(
                    "    subscribe(() => {{ __setNodeText({}, {}); }});\n",
                    dyn_var, formatted
                ));
                dyn_var
            }
            NodeType::IfBlock => {
                let if_container = self.next_var("if_cont");
                let scope_var = self.next_var("__scope");
                sb.push_str(&format!(
                    "    const {} = document.createElement(\"span\");\n",
                    if_container
                ));
                sb.push_str(&format!(
                    "    {}.style.display = \"contents\";\n",
                    if_container
                ));
                sb.push_str(&format!(
                    "    const {} = __curr_scope + \"/if_\" + (++__scope_seq);\n",
                    scope_var
                ));

                let cond_expr = node.condition.as_deref().unwrap_or("true");
                let formatted_cond = self.format_expr(cond_expr);

                let render_fn = self.next_var("__render_if");
                let prev_scope = self.next_var("__prev_scope");
                sb.push_str(&format!("    function {}() {{\n", render_fn));
                sb.push_str(&format!("      __clear_scope({});\n", scope_var));
                sb.push_str(&format!("      const {} = __curr_scope;\n", prev_scope));
                sb.push_str(&format!("      __curr_scope = {};\n", scope_var));
                sb.push_str("      try {\n");
                sb.push_str(&format!("      {}.innerHTML = \"\";\n", if_container));
                sb.push_str(&format!("      if ({}) {{\n", formatted_cond));

                for child in &node.consequent {
                    let child_var = self.emit_node(sb, child);
                    sb.push_str(&format!("        {}.appendChild({});\n", if_container, child_var));
                }

                if !node.alternate.is_empty() {
                    sb.push_str("      } else {\n");
                    for child in &node.alternate {
                        let child_var = self.emit_node(sb, child);
                        sb.push_str(&format!("        {}.appendChild({});\n", if_container, child_var));
                    }
                }
                sb.push_str("      }\n");
                sb.push_str(&format!("      }} finally {{ __curr_scope = {}; }}\n", prev_scope));
                sb.push_str("    }\n");

                sb.push_str(&format!("    {}();\n", render_fn));
                sb.push_str(&format!("    subscribe({});\n", render_fn));

                if_container
            }
            NodeType::EachBlock => {
                let each_container = self.next_var("each_cont");
                let scope_var = self.next_var("__scope");
                sb.push_str(&format!(
                    "    const {} = document.createElement(\"span\");\n",
                    each_container
                ));
                sb.push_str(&format!(
                    "    {}.style.display = \"contents\";\n",
                    each_container
                ));
                sb.push_str(&format!(
                    "    const {} = __curr_scope + \"/each_\" + (++__scope_seq);\n",
                    scope_var
                ));

                let col_expr = node.collection.as_deref().unwrap_or("[]");
                let formatted_col = self.format_expr(col_expr);
                let item_name = node.item_name.as_deref().unwrap_or("item");
                let index_name = node.index_name.as_deref();

                let render_fn = self.next_var("__render_each");
                let prev_scope = self.next_var("__prev_scope");
                sb.push_str(&format!("    function {}() {{\n", render_fn));
                sb.push_str(&format!("      __clear_scope({});\n", scope_var));
                sb.push_str(&format!("      const {} = __curr_scope;\n", prev_scope));
                sb.push_str(&format!("      __curr_scope = {};\n", scope_var));
                sb.push_str("      try {\n");
                sb.push_str(&format!("      {}.innerHTML = \"\";\n", each_container));
                sb.push_str(&format!(
                    "      const __list = {} || [];\n",
                    formatted_col
                ));
                sb.push_str("      for (let __idx = 0; __idx < __list.length; __idx++) {\n");
                sb.push_str(&format!("        const {} = __list[__idx];\n", item_name));
                if let Some(idx_name) = index_name {
                    sb.push_str(&format!("        const {} = __idx;\n", idx_name));
                }

                self.local_vars.push(item_name.to_string());
                if let Some(idx_name) = index_name {
                    self.local_vars.push(idx_name.to_string());
                }
                for child in &node.children {
                    let child_var = self.emit_node(sb, child);
                    sb.push_str(&format!("        {}.appendChild({});\n", each_container, child_var));
                }
                if index_name.is_some() {
                    self.local_vars.pop();
                }
                self.local_vars.pop();
                sb.push_str("      }\n");
                sb.push_str(&format!("      }} finally {{ __curr_scope = {}; }}\n", prev_scope));
                sb.push_str("    }\n");

                sb.push_str(&format!("    {}();\n", render_fn));
                sb.push_str(&format!("    subscribe({});\n", render_fn));

                each_container
            }
        }
    }

    pub fn generate(&mut self) -> CodegenOutput {
        self.generate_with_styles(
            &crate::config::types::TailwindConfig::default(),
            &crate::config::types::UnoCssConfig::default(),
            None,
        )
        .unwrap_or_else(|_| CodegenOutput {
            html: self.generate_html(),
            js: self.generate_js(),
        })
    }

    pub fn generate_with_tailwind(
        &mut self,
        tailwind_cfg: &crate::config::types::TailwindConfig,
        project_dir: Option<&std::path::Path>,
    ) -> Result<CodegenOutput, String> {
        let tw = crate::compiler::tailwind::compile_tailwind(
            self.component,
            tailwind_cfg,
            project_dir,
        )?;
        let html = self.generate_html_with_css(
            Some(&tw.tailwind_base),
            &tw.processed_custom_style,
            Some(&tw.tailwind_utilities),
        );
        let js = self.generate_js();
        Ok(CodegenOutput { html, js })
    }

    pub fn generate_with_styles(
        &mut self,
        tailwind_cfg: &crate::config::types::TailwindConfig,
        unocss_cfg: &crate::config::types::UnoCssConfig,
        project_dir: Option<&std::path::Path>,
    ) -> Result<CodegenOutput, String> {
        let styles = crate::compiler::unocss::compile_all_styles(
            self.component,
            tailwind_cfg,
            unocss_cfg,
            project_dir,
        )?;
        Ok(self.generate_with_prebuilt_styles(&styles, unocss_cfg))
    }

    pub fn generate_with_prebuilt_styles(
        &mut self,
        styles: &crate::compiler::unocss::UnifiedCssOutput,
        unocss_cfg: &crate::config::types::UnoCssConfig,
    ) -> CodegenOutput {
        self.expand_variant_groups =
            unocss_cfg.enabled && unocss_cfg.mode != "off" && unocss_cfg.variant_group;
        let html = self.generate_html_with_css(
            Some(&styles.base_css),
            &styles.processed_custom_style,
            Some(&styles.utilities_css),
        );
        let js = self.generate_js();
        CodegenOutput { html, js }
    }
}
