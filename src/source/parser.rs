use super::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedEOF,
    InvalidTag(String),
    MismatchedTag { expected: String, found: String },
    UnterminatedScript,
    UnterminatedState,
    UnterminatedStyle,
    InvalidStateDeclaration(String),
    MissingScriptAttribute,
    UnterminatedIfBlock,
    UnterminatedEachBlock,
    InvalidEachSyntax(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEOF => write!(f, "unexpected end of file"),
            ParseError::InvalidTag(tag) => write!(f, "invalid HTML tag '<{}>'", tag),
            ParseError::MismatchedTag { expected, found } => {
                write!(f, "mismatched closing tag: expected '</{}>', found '</{}>'", expected, found)
            }
            ParseError::UnterminatedScript => write!(f, "unterminated <script> block"),
            ParseError::UnterminatedState => write!(f, "unterminated <state> block"),
            ParseError::UnterminatedStyle => write!(f, "unterminated <style> block"),
            ParseError::InvalidStateDeclaration(line) => write!(f, "invalid state declaration: '{}'", line),
            ParseError::MissingScriptAttribute => write!(f, "missing script attribute"),
            ParseError::UnterminatedIfBlock => write!(f, "unterminated '{{#if}}' block"),
            ParseError::UnterminatedEachBlock => write!(f, "unterminated '{{#each}}' block"),
            ParseError::InvalidEachSyntax(s) => write!(f, "invalid each block syntax: '{}'", s),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser<'a> {
    _source: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            _source: source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(c) = self.peek() {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn starts_with(&self, prefix: &str) -> bool {
        let prefix_chars: Vec<char> = prefix.chars().collect();
        if self.pos + prefix_chars.len() > self.chars.len() {
            return false;
        }
        for (i, &c) in prefix_chars.iter().enumerate() {
            if self.chars[self.pos + i] != c {
                return false;
            }
        }
        true
    }

    fn consume_prefix(&mut self, prefix: &str) -> bool {
        if self.starts_with(prefix) {
            for _ in 0..prefix.chars().count() {
                self.advance();
            }
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_html_comment(&mut self) -> Result<bool, ParseError> {
        if !self.starts_with("<!--") {
            return Ok(false);
        }
        self.consume_prefix("<!--");
        while self.pos < self.chars.len() {
            if self.starts_with("-->") {
                self.consume_prefix("-->");
                return Ok(true);
            }
            self.advance();
        }
        Err(ParseError::UnexpectedEOF)
    }

    pub fn parse(&mut self) -> Result<Component, ParseError> {
        let mut scripts = Vec::new();
        let mut states = Vec::new();
        let mut root_nodes = Vec::new();
        let mut styles = String::new();

        while self.pos < self.chars.len() {
            self.skip_whitespace();
            if self.pos >= self.chars.len() {
                break;
            }

            if self.starts_with("<!--") {
                self.skip_html_comment()?;
            } else if self.starts_with("<script") {
                let script = self.parse_script_block()?;
                scripts.push(script);
            } else if self.starts_with("<state>") {
                let parsed_states = self.parse_state_block()?;
                states.extend(parsed_states);
            } else if self.starts_with("<style>") {
                styles = self.parse_style_block()?;
            } else if self.starts_with("{#if") {
                let node = self.parse_if_block()?;
                root_nodes.push(node);
            } else if self.starts_with("{#each") {
                let node = self.parse_each_block()?;
                root_nodes.push(node);
            } else if self.peek() == Some('<') && !self.starts_with("</") {
                let node = self.parse_element()?;
                root_nodes.push(node);
            } else if self.peek() == Some('{') {
                let node = self.parse_interpolation()?;
                root_nodes.push(node);
            } else {
                let text = self.parse_text();
                if !text.trim().is_empty() {
                    root_nodes.push(Node {
                        node_type: NodeType::Text,
                        content: text,
                        line: self.line,
                        ..Default::default()
                    });
                }
            }
        }

        // Auto-extract top-level script variables into reactive state (Svelte-style!)
        for script in &scripts {
            Self::extract_script_state_variables(&mut states, script);
        }

        Ok(Component {
            scripts,
            states,
            root_nodes,
            styles,
        })
    }

    fn parse_script_block(&mut self) -> Result<ScriptBlock, ParseError> {
        let start_line = self.line;
        let start_col = self.col;
        self.consume_prefix("<script");

        let mut lang = None;
        let mut context = None;

        while self.pos < self.chars.len() && self.peek() != Some('>') {
            self.skip_whitespace();
            if self.peek() == Some('>') {
                break;
            }

            let mut attr_name = String::new();
            while let Some(c) = self.peek() {
                if c == '=' || c.is_whitespace() || c == '>' {
                    break;
                }
                attr_name.push(c);
                self.advance();
            }

            self.skip_whitespace();
            if self.peek() == Some('=') {
                self.advance();
                self.skip_whitespace();
                if matches!(self.peek(), Some('"') | Some('\'')) {
                    let quote = self.advance().unwrap();
                    let mut val = String::new();
                    while let Some(c) = self.peek() {
                        if c == quote {
                            break;
                        }
                        val.push(c);
                        self.advance();
                    }
                    if self.peek() == Some(quote) {
                        self.advance();
                    }

                    if attr_name == "lang" {
                        lang = ScriptLang::from_str(&val);
                    } else if attr_name == "context" {
                        context = ScriptContext::from_str(&val);
                    }
                }
            }
        }

        if self.peek() == Some('>') {
            self.advance();
        }

        // Apply sensible Svelte-style defaults if attributes are omitted
        if lang.is_none() {
            if let Some(ctx) = context {
                lang = match ctx {
                    ScriptContext::Server => Some(ScriptLang::Python),
                    ScriptContext::Native | ScriptContext::Wasm => Some(ScriptLang::Rust),
                    ScriptContext::Client => Some(ScriptLang::Js),
                };
            } else {
                lang = Some(ScriptLang::Js);
            }
        }

        if context.is_none() {
            context = match lang.unwrap() {
                ScriptLang::Python => Some(ScriptContext::Server),
                ScriptLang::Rust => Some(ScriptContext::Native),
                ScriptLang::Js => Some(ScriptContext::Client),
            };
        }

        let mut body = String::new();
        while self.pos < self.chars.len() {
            if self.starts_with("</script>") {
                self.consume_prefix("</script>");
                return Ok(ScriptBlock {
                    lang: lang.unwrap(),
                    context: context.unwrap(),
                    code: body.trim().to_string(),
                    line: start_line,
                    col: start_col,
                });
            }
            if let Some(c) = self.advance() {
                body.push(c);
            }
        }

        Err(ParseError::UnterminatedScript)
    }

    fn scan_quoted_string(s: &str) -> Option<(usize, char)> {
        let mut chars = s.char_indices();
        let (_, quote) = chars.next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let mut escaped = false;
        for (idx, c) in chars {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                return Some((idx, quote));
            }
        }
        None
    }

    fn normalize_safe_literal(raw: &str) -> Option<(String, String)> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed == "True" || trimmed == "true" {
            return Some(("true".to_string(), "bool".to_string()));
        }
        if trimmed == "False" || trimmed == "false" {
            return Some(("false".to_string(), "bool".to_string()));
        }
        if trimmed == "None" || trimmed == "null" {
            return Some(("null".to_string(), "any".to_string()));
        }
        // Quoted string literal covering entire trimmed slice (no trailing + / method calls)
        if let Some((end_idx, quote)) = Self::scan_quoted_string(trimmed) {
            if end_idx + quote.len_utf8() == trimmed.len() {
                let inner = &trimmed[1..end_idx];
                if inner.contains('\n') || inner.contains('\r') {
                    return None;
                }
                if quote == '\'' {
                    let converted = inner.replace("\\'", "'").replace('"', "\\\"");
                    return Some((format!("\"{}\"", converted), "string".to_string()));
                } else {
                    return Some((trimmed.to_string(), "string".to_string()));
                }
            }
            return None;
        }
        // Safe ASCII numeric literal
        if !trimmed.is_empty()
            && trimmed
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'-' || b == b'.')
            && trimmed.parse::<f64>().ok().is_some_and(|n| n.is_finite())
        {
            let type_name = if trimmed.contains('.') { "f32" } else { "i32" };
            return Some((trimmed.to_string(), type_name.to_string()));
        }
        // Simple literal array or object literal on single line (skip comprehensions or fn calls)
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                return Some(("[]".to_string(), "list".to_string()));
            }
            let items = Self::split_top_level_comma(inner)?;
            let mut normalized = Vec::with_capacity(items.len());
            for item in items {
                let (norm, _) = Self::normalize_safe_literal(item)?;
                normalized.push(norm);
            }
            return Some((format!("[{}]", normalized.join(", ")), "list".to_string()));
        }
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                return Some(("{}".to_string(), "dict".to_string()));
            }
            let pairs = Self::split_top_level_comma(inner)?;
            let mut normalized = Vec::with_capacity(pairs.len());
            for pair in pairs {
                let colon = Self::find_top_level_colon(pair)?;
                let (k_norm, k_ty) = Self::normalize_safe_literal(&pair[..colon])?;
                if k_ty != "string" {
                    return None;
                }
                let (v_norm, _) = Self::normalize_safe_literal(&pair[colon + 1..])?;
                normalized.push(format!("{}: {}", k_norm, v_norm));
            }
            return Some((format!("{{{}}}", normalized.join(", ")), "dict".to_string()));
        }
        None
    }

    fn split_top_level_comma(s: &str) -> Option<Vec<&str>> {
        let mut parts = Vec::new();
        let mut depth = 0usize;
        let mut in_quote: Option<char> = None;
        let mut escaped = false;
        let mut start = 0usize;

        for (idx, c) in s.char_indices() {
            if let Some(q) = in_quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    in_quote = None;
                }
            } else {
                match c {
                    '"' | '\'' => in_quote = Some(c),
                    '[' | '{' | '(' => depth += 1,
                    ']' | '}' | ')' => depth = depth.checked_sub(1)?,
                    ',' if depth == 0 => {
                        let piece = s[start..idx].trim();
                        if !piece.is_empty() {
                            parts.push(piece);
                        }
                        start = idx + 1;
                    }
                    _ => {}
                }
            }
        }
        if depth != 0 || in_quote.is_some() {
            return None;
        }
        let tail = s[start..].trim();
        if !tail.is_empty() {
            parts.push(tail);
        }
        Some(parts)
    }

    fn find_top_level_colon(s: &str) -> Option<usize> {
        let mut in_quote: Option<char> = None;
        let mut escaped = false;
        for (idx, c) in s.char_indices() {
            if let Some(q) = in_quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    in_quote = None;
                }
            } else if c == '"' || c == '\'' {
                in_quote = Some(c);
            } else if c == ':' {
                return Some(idx);
            }
        }
        None
    }

    fn extract_script_state_variables(states: &mut Vec<StateVar>, script: &ScriptBlock) {
        for raw_line in script.code.lines() {
            if raw_line.is_empty() || raw_line.starts_with(' ') || raw_line.starts_with('\t') {
                continue;
            }
            let line = raw_line.trim();
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("//")
                || line.starts_with("/*")
                || line.starts_with("import ")
                || line.starts_with("from ")
                || line.starts_with("def ")
                || line.starts_with("async def ")
                || line.starts_with("class ")
                || line.starts_with("pub ")
                || line.starts_with("fn ")
                || line.starts_with("return ")
                || line.starts_with("use ")
                || line.starts_with("extern ")
            {
                continue;
            }

            let mut clean_line = line;
            if script.lang == ScriptLang::Js {
                if let Some(stripped) = clean_line.strip_prefix("let ") {
                    clean_line = stripped.trim();
                } else if let Some(stripped) = clean_line.strip_prefix("var ") {
                    clean_line = stripped.trim();
                } else if let Some(stripped) = clean_line.strip_prefix("const ") {
                    clean_line = stripped.trim();
                }
            } else if script.lang == ScriptLang::Rust {
                if let Some(stripped) = clean_line.strip_prefix("let mut ") {
                    clean_line = stripped.trim();
                } else if let Some(stripped) = clean_line.strip_prefix("let ") {
                    clean_line = stripped.trim();
                } else if let Some(stripped) = clean_line.strip_prefix("const ") {
                    clean_line = stripped.trim();
                } else if let Some(stripped) = clean_line.strip_prefix("static ") {
                    clean_line = stripped.trim();
                }
            }

            let eq_idx = match clean_line.find('=') {
                Some(idx) => idx,
                None => continue,
            };

            // Prevent comparison operators ==, +=, -=, <=, >=, !=
            if eq_idx > 0 {
                let prev = clean_line.as_bytes()[eq_idx - 1] as char;
                if matches!(prev, '=' | '+' | '-' | '*' | '/' | '<' | '>' | '!') {
                    continue;
                }
            }
            if eq_idx + 1 < clean_line.len() && clean_line.as_bytes()[eq_idx + 1] as char == '=' {
                continue;
            }

            let mut raw_lhs = clean_line[..eq_idx].trim();
            let raw_rhs = clean_line[eq_idx + 1..].trim().trim_end_matches(';').trim();

            if let Some(colon_idx) = raw_lhs.find(':') {
                raw_lhs = raw_lhs[..colon_idx].trim();
            }

            if raw_lhs.is_empty() {
                continue;
            }
            let mut is_valid = true;
            for (i, c) in raw_lhs.chars().enumerate() {
                if i == 0 {
                    if !c.is_alphabetic() && c != '_' {
                        is_valid = false;
                        break;
                    }
                } else if !c.is_alphanumeric() && c != '_' {
                    is_valid = false;
                    break;
                }
            }
            if !is_valid {
                continue;
            }

            // Strictly auto-extract ONLY safe JSON/JS literals for server/native scripts
            // (so Python runtime expressions like os.getenv(...) NEVER leak or crash browser JS!)
            let (initial_val, type_name) = match Self::normalize_safe_literal(raw_rhs) {
                Some(pair) => pair,
                None => continue,
            };

            if !states.iter().any(|s| s.name == raw_lhs) {
                states.push(StateVar {
                    name: raw_lhs.to_string(),
                    type_name,
                    initial_value: initial_val,
                    line: script.line,
                });
            }
        }
    }

    fn parse_state_block(&mut self) -> Result<Vec<StateVar>, ParseError> {
        self.consume_prefix("<state>");
        let mut states = Vec::new();
        let mut body = String::new();

        while self.pos < self.chars.len() {
            if self.starts_with("</state>") {
                self.consume_prefix("</state>");
                for line in body.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                        continue;
                    }
                    let colon_idx = trimmed.find(':').ok_or_else(|| ParseError::InvalidStateDeclaration(trimmed.to_string()))?;
                    let eq_idx = trimmed.find('=').ok_or_else(|| ParseError::InvalidStateDeclaration(trimmed.to_string()))?;
                    if colon_idx >= eq_idx {
                        return Err(ParseError::InvalidStateDeclaration(trimmed.to_string()));
                    }

                    let name = trimmed[..colon_idx].trim().to_string();
                    let type_name = trimmed[colon_idx + 1..eq_idx].trim().to_string();
                    let initial_value = trimmed[eq_idx + 1..].trim().trim_end_matches(';').trim().to_string();

                    states.push(StateVar {
                        name,
                        type_name,
                        initial_value,
                        line: self.line,
                    });
                }
                return Ok(states);
            }
            if let Some(c) = self.advance() {
                body.push(c);
            }
        }

        Err(ParseError::UnterminatedState)
    }

    fn parse_style_block(&mut self) -> Result<String, ParseError> {
        self.consume_prefix("<style>");
        let mut styles = String::new();

        while self.pos < self.chars.len() {
            if self.starts_with("</style>") {
                self.consume_prefix("</style>");
                return Ok(styles.trim().to_string());
            }
            if let Some(c) = self.advance() {
                styles.push(c);
            }
        }

        Err(ParseError::UnterminatedStyle)
    }

    fn parse_element(&mut self) -> Result<Node, ParseError> {
        let start_line = self.line;
        self.advance(); // consume '<'

        let mut tag = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            tag.push(c);
            self.advance();
        }

        if tag.is_empty() {
            return Err(ParseError::InvalidTag(tag));
        }

        let mut attributes = Vec::new();
        let mut events = Vec::new();
        let mut bindings = Vec::new();

        while self.pos < self.chars.len() && self.peek() != Some('>') && !self.starts_with("/>") {
            self.skip_whitespace();
            if self.peek() == Some('>') || self.starts_with("/>") {
                break;
            }

            let mut attr_name = String::new();
            while let Some(c) = self.peek() {
                if c == '=' || c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                attr_name.push(c);
                self.advance();
            }

            self.skip_whitespace();
            if self.peek() == Some('=') {
                self.advance();
                self.skip_whitespace();

                if self.peek() == Some('{') {
                    let expr = self.read_braced_expression()?;

                    if let Some(prop) = attr_name.strip_prefix("bind:") {
                        bindings.push(Binding {
                            property: prop.to_string(),
                            variable: expr,
                            line: start_line,
                        });
                    } else if matches!(attr_name.as_str(), "click" | "input" | "change" | "submit" | "keydown" | "keyup") {
                        events.push(EventBinding {
                            event_name: attr_name,
                            expression: expr,
                            line: start_line,
                        });
                    } else {
                        attributes.push(Attribute {
                            name: attr_name,
                            value: format!("{{{}}}", expr),
                        });
                    }
                } else if matches!(self.peek(), Some('"') | Some('\'')) {
                    let quote = self.advance().unwrap();
                    let mut val = String::new();
                    while let Some(c) = self.peek() {
                        if c == quote {
                            self.advance();
                            break;
                        }
                        val.push(c);
                        self.advance();
                    }
                    attributes.push(Attribute {
                        name: attr_name,
                        value: val,
                    });
                }
            } else {
                attributes.push(Attribute {
                    name: attr_name,
                    value: String::new(),
                });
            }
        }

        let is_self_closing = if self.starts_with("/>") {
            self.consume_prefix("/>");
            true
        } else if self.peek() == Some('>') {
            self.advance();
            // Standard HTML void elements
            matches!(tag.as_str(), "input" | "img" | "br" | "hr" | "meta" | "link")
        } else {
            false
        };

        if is_self_closing {
            return Ok(Node {
                node_type: NodeType::Element,
                tag,
                attributes,
                events,
                bindings,
                children: Vec::new(),
                line: start_line,
                ..Default::default()
            });
        }

        let mut children = Vec::new();
        let closing_tag = format!("</{}>", tag);

        while self.pos < self.chars.len() {
            if self.starts_with(&closing_tag) {
                self.consume_prefix(&closing_tag);
                return Ok(Node {
                    node_type: NodeType::Element,
                    tag,
                    attributes,
                    events,
                    bindings,
                    children,
                    line: start_line,
                    ..Default::default()
                });
            } else if self.starts_with("</") {
                // Mismatched closing tag
                let mut found = String::new();
                for _ in 0..2 { self.advance(); }
                while let Some(c) = self.peek() {
                    if c == '>' { self.advance(); break; }
                    found.push(c);
                    self.advance();
                }
                return Err(ParseError::MismatchedTag {
                    expected: tag,
                    found,
                });
            } else if self.starts_with("<!--") {
                self.skip_html_comment()?;
            } else if self.starts_with("{#if") {
                let node = self.parse_if_block()?;
                children.push(node);
            } else if self.starts_with("{#each") {
                let node = self.parse_each_block()?;
                children.push(node);
            } else if self.peek() == Some('<') {
                let node = self.parse_element()?;
                children.push(node);
            } else if self.peek() == Some('{') {
                let node = self.parse_interpolation()?;
                children.push(node);
            } else {
                let text = self.parse_text();
                if !text.is_empty() {
                    children.push(Node {
                        node_type: NodeType::Text,
                        content: text,
                        line: self.line,
                        ..Default::default()
                    });
                }
            }
        }

        Err(ParseError::UnexpectedEOF)
    }

    fn read_braced_expression(&mut self) -> Result<String, ParseError> {
        self.advance(); // consume opening '{'
        let mut expr = String::new();
        let mut depth = 1usize;
        let mut in_quote: Option<char> = None;
        let mut escaped = false;

        while let Some(c) = self.peek() {
            if let Some(q) = in_quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    in_quote = None;
                }
            } else {
                match c {
                    '"' | '\'' | '`' => in_quote = Some(c),
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            self.advance();
                            return Ok(expr.trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
            expr.push(c);
            self.advance();
        }

        Err(ParseError::UnexpectedEOF)
    }

    fn parse_interpolation(&mut self) -> Result<Node, ParseError> {
        let line = self.line;
        let expr = self.read_braced_expression()?;
        Ok(Node {
            node_type: NodeType::DynamicText,
            content: expr,
            line,
            ..Default::default()
        })
    }

    fn parse_if_block(&mut self) -> Result<Node, ParseError> {
        let start_line = self.line;
        self.consume_prefix("{#if");
        self.skip_whitespace();

        let mut condition = String::new();
        while let Some(c) = self.peek() {
            if c == '}' {
                self.advance();
                break;
            }
            condition.push(c);
            self.advance();
        }

        let mut consequent = Vec::new();
        let mut alternate = Vec::new();
        let mut in_else = false;

        while self.pos < self.chars.len() {
            if self.starts_with("{/if}") {
                self.consume_prefix("{/if}");
                return Ok(Node {
                    node_type: NodeType::IfBlock,
                    condition: Some(condition.trim().to_string()),
                    consequent,
                    alternate,
                    line: start_line,
                    ..Default::default()
                });
            } else if self.starts_with("{:else}") {
                self.consume_prefix("{:else}");
                in_else = true;
            } else if self.starts_with("<!--") {
                self.skip_html_comment()?;
            } else if self.starts_with("{#if") {
                let sub = self.parse_if_block()?;
                if in_else { alternate.push(sub); } else { consequent.push(sub); }
            } else if self.starts_with("{#each") {
                let sub = self.parse_each_block()?;
                if in_else { alternate.push(sub); } else { consequent.push(sub); }
            } else if self.starts_with("</") {
                return Err(ParseError::UnterminatedIfBlock);
            } else if self.peek() == Some('<') {
                let sub = self.parse_element()?;
                if in_else { alternate.push(sub); } else { consequent.push(sub); }
            } else if self.peek() == Some('{') && !self.starts_with("{/") && !self.starts_with("{:") {
                let sub = self.parse_interpolation()?;
                if in_else { alternate.push(sub); } else { consequent.push(sub); }
            } else if self.peek() == Some('{') {
                return Err(ParseError::UnterminatedIfBlock);
            } else {
                let text = self.parse_text();
                if !text.is_empty() {
                    let node = Node {
                        node_type: NodeType::Text,
                        content: text,
                        line: self.line,
                        ..Default::default()
                    };
                    if in_else { alternate.push(node); } else { consequent.push(node); }
                }
            }
        }

        Err(ParseError::UnterminatedIfBlock)
    }

    fn parse_each_block(&mut self) -> Result<Node, ParseError> {
        let start_line = self.line;
        self.consume_prefix("{#each");
        self.skip_whitespace();

        let mut header = String::new();
        while let Some(c) = self.peek() {
            if c == '}' {
                self.advance();
                break;
            }
            header.push(c);
            self.advance();
        }

        // Parse: items as item (key)
        let as_idx = header.find(" as ").ok_or_else(|| ParseError::InvalidEachSyntax(header.clone()))?;
        let collection = header[..as_idx].trim().to_string();
        let rest = header[as_idx + 4..].trim();

        let (item_name, key_name) = if let Some(paren_idx) = rest.find('(') {
            let item = rest[..paren_idx].trim().to_string();
            let key = rest[paren_idx + 1..].trim_end_matches(')').trim().to_string();
            (item, Some(key))
        } else {
            (rest.to_string(), None)
        };

        let mut children = Vec::new();
        while self.pos < self.chars.len() {
            if self.starts_with("{/each}") {
                self.consume_prefix("{/each}");
                return Ok(Node {
                    node_type: NodeType::EachBlock,
                    collection: Some(collection),
                    item_name: Some(item_name),
                    key_name,
                    children,
                    line: start_line,
                    ..Default::default()
                });
            } else if self.starts_with("<!--") {
                self.skip_html_comment()?;
            } else if self.starts_with("{#if") {
                let sub = self.parse_if_block()?;
                children.push(sub);
            } else if self.starts_with("{#each") {
                let sub = self.parse_each_block()?;
                children.push(sub);
            } else if self.starts_with("</") {
                return Err(ParseError::UnterminatedEachBlock);
            } else if self.peek() == Some('<') {
                let sub = self.parse_element()?;
                children.push(sub);
            } else if self.peek() == Some('{') && !self.starts_with("{/") {
                let sub = self.parse_interpolation()?;
                children.push(sub);
            } else if self.peek() == Some('{') {
                return Err(ParseError::UnterminatedEachBlock);
            } else {
                let text = self.parse_text();
                if !text.is_empty() {
                    children.push(Node {
                        node_type: NodeType::Text,
                        content: text,
                        line: self.line,
                        ..Default::default()
                    });
                }
            }
        }

        Err(ParseError::UnterminatedEachBlock)
    }

    fn parse_text(&mut self) -> String {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == '<' || c == '{' {
                break;
            }
            text.push(c);
            self.advance();
        }
        text
    }
}

pub fn parse_component(source: &str) -> Result<Component, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse()
}
