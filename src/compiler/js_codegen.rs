use crate::source::ast::*;

pub struct CodegenOutput {
    pub html: String,
    pub js: String,
}

pub struct JsCodegen<'a> {
    component: &'a Component,
    project_name: &'a str,
    var_id: usize,
    _in_dynamic_block: bool,
}

impl<'a> JsCodegen<'a> {
    pub fn new(component: &'a Component, project_name: &'a str) -> Self {
        Self {
            component,
            project_name,
            var_id: 0,
            _in_dynamic_block: false,
        }
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
        let mut out = String::new();
        let chars: Vec<char> = expr.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            if c == '"' || c == '\'' || c == '`' {
                let quote = c;
                out.push(quote);
                i += 1;
                while i < chars.len() {
                    let sc = chars[i];
                    out.push(sc);
                    if sc == '\\' && i + 1 < chars.len() {
                        i += 1;
                        out.push(chars[i]);
                        i += 1;
                        continue;
                    }
                    i += 1;
                    if sc == quote {
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
                let is_prop_access = start > 0 && chars[start - 1] == '.';

                let is_state = !is_prop_access && self.component.states.iter().any(|s| s.name == ident);
                if is_state {
                    out.push_str("state.");
                    out.push_str(&ident);
                } else {
                    out.push_str(&ident);
                }
            } else {
                out.push(c);
                i += 1;
            }
        }

        out
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
        let style_block = if !self.component.styles.is_empty() {
            format!("  <style>\n{}\n  </style>\n", self.component.styles)
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

        // Emit state initializers
        for state in &self.component.states {
            sb.push_str(&format!("    {}: {},\n", state.name, state.initial_value));
        }

        sb.push_str(
            r#"  };

  let subscribers = [];
  let __curr_scope = "root";
  let __scope_seq = 0;
  function subscribe(fn) {
    subscribers.push({ fn: fn, scope: __curr_scope });
  }
  function __clear_scope(prefix) {
    subscribers = subscribers.filter(function(s) {
      return s.scope !== prefix && !s.scope.startsWith(prefix + "/");
    });
  }
  function update() {
    const snapshot = subscribers.slice();
    for (let i = 0; i < snapshot.length; i++) {
      snapshot[i].fn();
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

        // Inject user client-side JavaScript scripts
        for script in &self.component.scripts {
            if script.lang == ScriptLang::Js && script.context == ScriptContext::Client {
                sb.push_str("\n  // User Client Script\n  ");
                sb.push_str(&script.code);
                sb.push_str("\n");
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
                    if attr.value.starts_with('{') && attr.value.ends_with('}') && attr.value.len() >= 2 {
                        let expr = &attr.value[1..attr.value.len() - 1];
                        let formatted = self.format_expr(expr);
                        sb.push_str(&format!(
                            "    {}.setAttribute(\"{}\", {});\n",
                            el_var, attr.name, formatted
                        ));
                        sb.push_str(&format!(
                            "    subscribe(() => {{ {}.setAttribute(\"{}\", {}); }});\n",
                            el_var, attr.name, formatted
                        ));
                    } else {
                        sb.push_str(&format!(
                            "    {}.setAttribute(\"{}\", \"{}\");\n",
                            el_var,
                            Self::escape_js_string(&attr.name),
                            Self::escape_js_string(&attr.value)
                        ));
                    }
                }

                // Two-way bindings (bind:value / bind:checked)
                for binding in &node.bindings {
                    let var_name = &binding.variable;
                    let is_state = self.component.states.iter().any(|s| s.name == *var_name);
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

                // Events
                for event in &node.events {
                    let formatted = self.format_expr(&event.expression);
                    sb.push_str(&format!(
                        "    {}.addEventListener(\"{}\", function(event) {{ {}; update(); }});\n",
                        el_var, event.event_name, formatted
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

                for child in &node.children {
                    let child_var = self.emit_node(sb, child);
                    sb.push_str(&format!("        {}.appendChild({});\n", each_container, child_var));
                }
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
        let html = self.generate_html();
        let js = self.generate_js();
        CodegenOutput { html, js }
    }
}
