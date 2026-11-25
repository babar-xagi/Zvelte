use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use crate::compiler::tailwind::{
    breakpoint_weight_and_query, build_rule_with_variants, compile_tailwind,
    escape_css_ident, extract_quoted_tokens_from_expr, keyframe_css,
    resolve_color, resolve_radius, resolve_spacing, resolve_utility,
    split_variants_outside_brackets, ResolvedUtility, PREFLIGHT_CSS,
};
use crate::config::types::{TailwindConfig, UnoCssConfig};
use crate::source::ast::{Component, Node, NodeType};

/// Combined CSS compilation result across Tailwind CSS and UnoCSS.
pub struct UnifiedCssOutput {
    pub processed_custom_style: String,
    pub base_css: String,
    pub utilities_css: String,
    pub tailwind_utilities: usize,
    pub unocss_rules: usize,
}

#[derive(Clone, Debug)]
struct CandidateMeta {
    token: String,
    from_variant_group: bool,
}

#[derive(Clone, Debug)]
struct AttributifyEntry {
    attr_name: String,
    value_token: Option<String>,
}

const ATTRIBUTIFY_PREFIXES: &[&str] = &[
    "flex", "grid", "p", "pa", "m", "ma", "space", "gap", "text", "c", "color", "font", "fw",
    "fs", "lh", "ws", "ta", "bg", "border", "b", "bx", "by", "bt", "br", "bb", "bl", "rounded",
    "rd", "shadow", "ring", "op", "opacity", "divide", "from", "via", "to", "w", "h", "min-w",
    "min-h", "max-w", "max-h", "size", "pos", "inset", "top", "right", "bottom", "left", "z",
    "overflow", "of", "transition", "duration", "delay", "animate", "cursor", "cur", "select",
    "scale", "rotate", "translate", "items", "justify", "content", "self", "place", "col",
    "row", "aspect", "backdrop", "blur", "brightness", "contrast", "drop-shadow",
];

const VARIANT_PREFIXES: &[&str] = &[
    "hover", "focus", "focus-visible", "focus-within", "active", "visited", "disabled",
    "enabled", "checked", "first", "last", "odd", "even", "open", "dark", "sm", "md", "lg",
    "xl", "2xl", "max-sm", "max-md", "max-lg", "max-xl", "max-2xl", "lt-sm", "lt-md", "lt-lg",
    "lt-xl", "lt-2xl", "group-hover", "group-focus", "group-active", "peer-checked",
    "peer-focus", "before", "after", "placeholder", "hocus", "children", "not-first",
    "not-last", "not-disabled", "not-checked",
];

fn strip_unocss_prefix(s: &str) -> (&str, bool) {
    if let Some(rest) = s.strip_prefix("un-") {
        (rest, true)
    } else {
        (s, false)
    }
}

/// Checks if an attribute name is a standard HTML/DOM/event attribute that must NOT trigger UnoCSS Attributify.
pub fn is_standard_html_attr(tag: &str, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("on")
        || lower.starts_with("aria-")
        || lower.starts_with("data-")
        || lower.starts_with("bind:")
    {
        return true;
    }

    if (tag == "input" || tag == "select") && lower == "size" {
        return true;
    }

    if matches!(
        tag,
        "svg" | "path" | "circle" | "rect" | "line" | "polygon" | "polyline" | "ellipse" | "g"
            | "defs" | "use"
    ) && matches!(
        lower.as_str(),
        "xmlns" | "viewbox" | "fill" | "stroke" | "stroke-width" | "stroke-linecap"
            | "stroke-linejoin" | "d" | "cx" | "cy" | "r" | "x" | "y" | "x1" | "y1" | "x2"
            | "y2" | "points"
    ) {
        return true;
    }

    matches!(
        lower.as_str(),
        "type"
            | "id"
            | "name"
            | "href"
            | "src"
            | "alt"
            | "title"
            | "value"
            | "placeholder"
            | "class"
            | "style"
            | "for"
            | "action"
            | "method"
            | "rel"
            | "target"
            | "role"
            | "tabindex"
            | "disabled"
            | "checked"
            | "readonly"
            | "required"
            | "multiple"
            | "selected"
            | "hidden"
            | "open"
            | "autofocus"
            | "controls"
            | "loop"
            | "muted"
            | "rows"
            | "cols"
            | "colspan"
            | "rowspan"
            | "min"
            | "max"
            | "step"
            | "pattern"
            | "width"
            | "height"
            | "accept"
            | "autocomplete"
            | "enctype"
            | "novalidate"
            | "download"
            | "lang"
            | "dir"
            | "slot"
            | "part"
            | "is"
            | "draggable"
            | "spellcheck"
            | "contenteditable"
    )
}

/// Returns true if an element attribute is a recognized grouped UnoCSS Attributify attribute.
pub fn is_grouped_attributify_attr(tag: &str, attr_name: &str) -> bool {
    if is_standard_html_attr(tag, attr_name) {
        return false;
    }
    let (clean, had_un) = strip_unocss_prefix(attr_name);
    had_un || ATTRIBUTIFY_PREFIXES.contains(&clean) || VARIANT_PREFIXES.contains(&clean)
}

fn split_group_tokens(input: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '[' => {
                if start.is_none() {
                    start = Some(idx);
                }
                bracket_depth += 1;
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
            }
            '(' if bracket_depth == 0 => {
                if start.is_none() {
                    start = Some(idx);
                }
                paren_depth += 1;
            }
            ')' if bracket_depth == 0 => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            c if c.is_ascii_whitespace() && paren_depth == 0 && bracket_depth == 0 => {
                if let Some(s) = start.take() {
                    tokens.push(&input[s..idx]);
                }
            }
            _ => {
                if start.is_none() {
                    start = Some(idx);
                }
            }
        }
    }
    if let Some(s) = start {
        tokens.push(&input[s..]);
    }
    tokens
}

fn expand_single_token(tok: &str, depth: usize) -> Vec<String> {
    if depth >= 16 {
        return vec![tok.to_string()];
    }

    let bytes = tok.as_bytes();
    let mut bracket_depth = 0usize;
    let mut open_idx: Option<usize> = None;

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b'(' if bracket_depth == 0 && i > 0 && bytes[i - 1] == b':' => {
                open_idx = Some(i);
                break;
            }
            _ => {}
        }
    }

    let Some(lparen) = open_idx else {
        return vec![tok.to_string()];
    };

    let prefix = &tok[..lparen - 1]; // strip trailing ':'
    if prefix.is_empty() {
        return vec![tok.to_string()];
    }

    let mut paren_depth = 0usize;
    let mut b_depth = 0usize;
    let mut close_idx: Option<usize> = None;

    for (i, &b) in bytes.iter().enumerate().skip(lparen) {
        match b {
            b'[' => b_depth += 1,
            b']' => b_depth = b_depth.saturating_sub(1),
            b'(' if b_depth == 0 => paren_depth += 1,
            b')' if b_depth == 0 => {
                paren_depth = paren_depth.saturating_sub(1);
                if paren_depth == 0 {
                    close_idx = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(rparen) = close_idx else {
        return vec![tok.to_string()];
    };

    let suffix = &tok[rparen + 1..];
    if !suffix.is_empty() && suffix != "!" {
        return vec![tok.to_string()];
    }

    let inner = &tok[lparen + 1..rparen];
    let mut out = Vec::new();
    for piece in split_group_tokens(inner) {
        for expanded in expand_single_token(piece, depth + 1) {
            if suffix == "!" && !expanded.ends_with('!') {
                out.push(format!("{}:{}!", prefix, expanded));
            } else {
                out.push(format!("{}:{}", prefix, expanded));
            }
        }
    }

    if out.is_empty() {
        vec![tok.to_string()]
    } else {
        out
    }
}

/// Expands UnoCSS variant groups such as `hover:(bg-sky-500 text-white)` into flat class tokens.
/// Leaves arbitrary values like `bg-[rgb(1,2,3)]` or `[color:var(--x)]` untouched.
pub fn expand_variant_groups(input: &str) -> String {
    let mut expanded = Vec::new();
    for tok in split_group_tokens(input) {
        expanded.extend(expand_single_token(tok, 0));
    }
    expanded.join(" ")
}

/// Expands variant groups inside string literals and template chunks of a JS expression.
pub fn expand_variant_groups_in_expr(expr: &str) -> String {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            let quote = c;
            out.push(quote);
            i += 1;
            let mut literal = String::new();
            while i < chars.len() {
                let sc = chars[i];
                if sc == '\\' && i + 1 < chars.len() {
                    literal.push(sc);
                    i += 1;
                    literal.push(chars[i]);
                    i += 1;
                    continue;
                }
                if sc == quote {
                    i += 1;
                    break;
                }
                literal.push(sc);
                i += 1;
            }
            out.push_str(&expand_variant_groups(&literal));
            out.push(quote);
        } else if c == '`' {
            out.push('`');
            i += 1;
            let mut chunk = String::new();
            while i < chars.len() {
                let sc = chars[i];
                if sc == '\\' && i + 1 < chars.len() {
                    chunk.push(sc);
                    i += 1;
                    chunk.push(chars[i]);
                    i += 1;
                    continue;
                }
                if sc == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                    if !chunk.is_empty() {
                        let lead_ws: String =
                            chunk.chars().take_while(|ch| ch.is_ascii_whitespace()).collect();
                        let trail_ws: String = chunk
                            .chars()
                            .rev()
                            .take_while(|ch| ch.is_ascii_whitespace())
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect();
                        let trimmed = chunk.trim();
                        out.push_str(&lead_ws);
                        if !trimmed.is_empty() {
                            out.push_str(&expand_variant_groups(trimmed));
                        }
                        if !trimmed.is_empty() || lead_ws.is_empty() {
                            out.push_str(&trail_ws);
                        }
                        chunk.clear();
                    }
                    out.push_str("${");
                    i += 2;
                    let mut inner_expr = String::new();
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
                        inner_expr.push(ic);
                        i += 1;
                    }
                    out.push_str(&expand_variant_groups_in_expr(&inner_expr));
                    out.push('}');
                    continue;
                }
                if sc == '`' {
                    i += 1;
                    break;
                }
                chunk.push(sc);
                i += 1;
            }
            if !chunk.is_empty() {
                let lead_ws: String = chunk.chars().take_while(|ch| ch.is_ascii_whitespace()).collect();
                let trail_ws: String = chunk
                    .chars()
                    .rev()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                let trimmed = chunk.trim();
                out.push_str(&lead_ws);
                if !trimmed.is_empty() {
                    out.push_str(&expand_variant_groups(trimmed));
                }
                if !trimmed.is_empty() || lead_ws.is_empty() {
                    out.push_str(&trail_ws);
                }
            }
            out.push('`');
        } else {
            out.push(c);
            i += 1;
        }
    }

    out
}

/// Resolves UnoCSS preset-uno / preset-wind utilities plus standard Tailwind utilities.
/// Returns `(resolved_utility, is_unocss_specific)`.
pub(crate) fn resolve_unocss_utility(raw_utility: &str) -> Option<(ResolvedUtility, bool)> {
    let (clean, had_un) = strip_unocss_prefix(raw_utility);
    if let Some(tw) = resolve_utility(clean) {
        return Some((tw, had_un));
    }

    // Flex / Grid preset conveniences
    let preset_layout = match clean {
        "flex-center" => Some(ResolvedUtility::many(vec![
            ("display", "flex".into()),
            ("justify-content", "center".into()),
            ("align-items", "center".into()),
        ])),
        "inline-flex-center" => Some(ResolvedUtility::many(vec![
            ("display", "inline-flex".into()),
            ("justify-content", "center".into()),
            ("align-items", "center".into()),
        ])),
        "flex-col-center" => Some(ResolvedUtility::many(vec![
            ("display", "flex".into()),
            ("flex-direction", "column".into()),
            ("justify-content", "center".into()),
            ("align-items", "center".into()),
        ])),
        "flex-between" => Some(ResolvedUtility::many(vec![
            ("display", "flex".into()),
            ("justify-content", "space-between".into()),
            ("align-items", "center".into()),
        ])),
        "flex-around" => Some(ResolvedUtility::many(vec![
            ("display", "flex".into()),
            ("justify-content", "space-around".into()),
            ("align-items", "center".into()),
        ])),
        "grid-center" => Some(ResolvedUtility::many(vec![
            ("display", "grid".into()),
            ("place-items", "center".into()),
        ])),
        "ma" | "m-a" => Some(ResolvedUtility::one("margin", "auto")),
        "mxa" | "mx-a" | "m-x-a" => Some(ResolvedUtility::many(vec![
            ("margin-left", "auto".into()),
            ("margin-right", "auto".into()),
        ])),
        "mya" | "my-a" | "m-y-a" => Some(ResolvedUtility::many(vec![
            ("margin-top", "auto".into()),
            ("margin-bottom", "auto".into()),
        ])),
        "mta" | "mt-a" | "m-t-a" => Some(ResolvedUtility::one("margin-top", "auto")),
        "mba" | "mb-a" | "m-b-a" => Some(ResolvedUtility::one("margin-bottom", "auto")),
        "mla" | "ml-a" | "m-l-a" => Some(ResolvedUtility::one("margin-left", "auto")),
        "mra" | "mr-a" | "m-r-a" => Some(ResolvedUtility::one("margin-right", "auto")),
        "fs-italic" => Some(ResolvedUtility::one("font-style", "italic")),
        "fs-normal" => Some(ResolvedUtility::one("font-style", "normal")),
        "ta-left" => Some(ResolvedUtility::one("text-align", "left")),
        "ta-center" => Some(ResolvedUtility::one("text-align", "center")),
        "ta-right" => Some(ResolvedUtility::one("text-align", "right")),
        "ta-justify" => Some(ResolvedUtility::one("text-align", "justify")),
        "ta-start" => Some(ResolvedUtility::one("text-align", "start")),
        "ta-end" => Some(ResolvedUtility::one("text-align", "end")),
        "pos-relative" => Some(ResolvedUtility::one("position", "relative")),
        "pos-absolute" => Some(ResolvedUtility::one("position", "absolute")),
        "pos-fixed" => Some(ResolvedUtility::one("position", "fixed")),
        "pos-sticky" => Some(ResolvedUtility::one("position", "sticky")),
        "pos-static" => Some(ResolvedUtility::one("position", "static")),
        "rd" => resolve_utility("rounded"),
        "b" => resolve_utility("border"),
        "bx" => resolve_utility("border-x"),
        "by" => resolve_utility("border-y"),
        "bt" => resolve_utility("border-t"),
        "br" => resolve_utility("border-r"),
        "bb" => resolve_utility("border-b"),
        "bl" => resolve_utility("border-l"),
        _ => None,
    };
    if let Some(res) = preset_layout {
        return Some((res, true));
    }

    // Prefix shorthand delegations into tailwind::resolve_utility
    let delegated = if let Some(rest) = clean.strip_prefix("pa-") {
        Some(format!("p-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("c-") {
        Some(format!("text-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("color-") {
        Some(format!("text-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("op-") {
        Some(format!("opacity-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("fw-") {
        Some(format!("font-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("lh-") {
        Some(format!("leading-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("ws-") {
        Some(format!("whitespace-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("cur-") {
        Some(format!("cursor-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("of-") {
        Some(format!("overflow-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("rd-") {
        Some(format!("rounded-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("pos-") {
        Some(rest.to_string())
    } else if let Some(rest) = clean.strip_prefix("b-") {
        Some(format!("border-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("bx-") {
        Some(format!("border-x-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("by-") {
        Some(format!("border-y-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("bt-") {
        Some(format!("border-t-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("br-") {
        Some(format!("border-r-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("bb-") {
        Some(format!("border-b-{}", rest))
    } else if let Some(rest) = clean.strip_prefix("bl-") {
        Some(format!("border-l-{}", rest))
    } else {
        None
    };

    if let Some(target) = delegated {
        if let Some(res) = resolve_utility(&target) {
            return Some((res, true));
        }
    }

    None
}

fn lt_breakpoint_query(bp: &str) -> Option<&'static str> {
    match bp {
        "lt-sm" => Some("@media (max-width: 639.9px)"),
        "lt-md" => Some("@media (max-width: 767.9px)"),
        "lt-lg" => Some("@media (max-width: 1023.9px)"),
        "lt-xl" => Some("@media (max-width: 1279.9px)"),
        "lt-2xl" => Some("@media (max-width: 1535.9px)"),
        _ => None,
    }
}

/// Strips leading and/or trailing `!` important markers from a raw token or utility segment.
fn parse_important_and_core<'a>(parts: &[&'a str]) -> Option<(bool, &'a str)> {
    let raw_last = *parts.last()?;
    let mut important = false;
    let mut core = raw_last;

    if let Some(stripped) = core.strip_prefix('!') {
        important = true;
        core = stripped;
    }
    if let Some(stripped) = core.strip_suffix('!') {
        important = true;
        core = stripped;
    }
    if parts.first().is_some_and(|p| p.starts_with('!')) {
        important = true;
    }
    if core.is_empty() {
        return None;
    }
    Some((important, core))
}

fn normalize_variants<'a>(
    raw_variants: &[&'a str],
    resolved: &mut ResolvedUtility,
) -> (Vec<&'a str>, Vec<&'static str>, bool) {
    let mut normalized: Vec<&'a str> = Vec::with_capacity(raw_variants.len());
    let mut lt_queries: Vec<&'static str> = Vec::new();
    let mut used_uno_variant = false;

    for &v in raw_variants {
        let (clean_v, had_un) = strip_unocss_prefix(v);
        let clean_v = clean_v.strip_prefix('!').unwrap_or(clean_v);
        if had_un {
            used_uno_variant = true;
        }
        if let Some(mq) = lt_breakpoint_query(clean_v) {
            lt_queries.push(mq);
            used_uno_variant = true;
        } else if clean_v == "hocus" {
            normalized.push("[&:is(:hover,:focus)]");
            used_uno_variant = true;
        } else if clean_v == "not-first" {
            normalized.push("[&:not(:first-child)]");
            used_uno_variant = true;
        } else if clean_v == "not-last" {
            normalized.push("[&:not(:last-child)]");
            used_uno_variant = true;
        } else if clean_v == "not-disabled" {
            normalized.push("[&:not(:disabled)]");
            used_uno_variant = true;
        } else if clean_v == "not-checked" {
            normalized.push("[&:not(:checked)]");
            used_uno_variant = true;
        } else if clean_v == "children" {
            if resolved.child_combinator.is_none() {
                resolved.child_combinator = Some(" > *");
            }
            used_uno_variant = true;
        } else if clean_v == "all" {
            if resolved.child_combinator.is_none() {
                resolved.child_combinator = Some(" *");
            }
            used_uno_variant = true;
        } else {
            normalized.push(clean_v);
        }
    }

    (normalized, lt_queries, used_uno_variant)
}

fn compile_rule_with_all_variants(
    selector_base: &str,
    raw_variants: &[&str],
    important: bool,
    mut resolved: ResolvedUtility,
    dark_mode: &str,
) -> Option<(u8, Vec<String>, String, Option<&'static str>, bool)> {
    let (norm_vars, lt_queries, used_uno_variant) = normalize_variants(raw_variants, &mut resolved);
    let (mut weight, mut mqs, rule, kf) =
        build_rule_with_variants(selector_base, &norm_vars, important, resolved, dark_mode)?;

    if !lt_queries.is_empty() {
        weight = weight.max(1);
        for mq in lt_queries {
            mqs.push(mq.to_string());
        }
    }

    Some((weight, mqs, rule, kf, used_uno_variant))
}

fn push_class_tokens(
    raw: &str,
    out: &mut Vec<CandidateMeta>,
    seen: &mut HashSet<String>,
    enable_groups: bool,
) {
    if enable_groups {
        for tok in split_group_tokens(raw) {
            let expanded = expand_single_token(tok, 0);
            let had_group = expanded.len() > 1 || expanded.first().is_some_and(|e| e != tok);
            for piece in expanded {
                if seen.insert(piece.clone()) {
                    out.push(CandidateMeta {
                        token: piece,
                        from_variant_group: had_group,
                    });
                }
            }
        }
    } else {
        for tok in raw.split_whitespace() {
            if seen.insert(tok.to_string()) {
                out.push(CandidateMeta {
                    token: tok.to_string(),
                    from_variant_group: false,
                });
            }
        }
    }
}

fn push_expr_class_candidates(
    expr: &str,
    enable_groups: bool,
    out: &mut Vec<CandidateMeta>,
    seen: &mut HashSet<String>,
) {
    let mut orig_tokens = Vec::new();
    let mut orig_seen = HashSet::new();
    extract_quoted_tokens_from_expr(expr, &mut orig_tokens, &mut orig_seen);

    if enable_groups {
        let expanded_expr = expand_variant_groups_in_expr(expr);
        let mut exp_tokens = Vec::new();
        let mut exp_seen = HashSet::new();
        extract_quoted_tokens_from_expr(&expanded_expr, &mut exp_tokens, &mut exp_seen);
        for tok in exp_tokens {
            let from_group = !orig_seen.contains(&tok);
            if seen.insert(tok.clone()) {
                out.push(CandidateMeta {
                    token: tok,
                    from_variant_group: from_group,
                });
            }
        }
    } else {
        for rt in orig_tokens {
            push_class_tokens(&rt, out, seen, false);
        }
    }
}

fn collect_attributify_candidates(
    nodes: &[Node],
    config: &UnoCssConfig,
    class_candidates: &mut Vec<CandidateMeta>,
    class_seen: &mut HashSet<String>,
    attr_candidates: &mut Vec<AttributifyEntry>,
    attr_seen: &mut HashSet<String>,
) {
    for node in nodes {
        let tag = node.tag.as_str();
        for attr in &node.attributes {
            if attr.name == "class" {
                if attr.value.starts_with('{') && attr.value.ends_with('}') {
                    push_expr_class_candidates(
                        &attr.value,
                        config.variant_group,
                        class_candidates,
                        class_seen,
                    );
                } else {
                    push_class_tokens(
                        &attr.value,
                        class_candidates,
                        class_seen,
                        config.variant_group,
                    );
                }
                continue;
            }

            if !config.attributify || is_standard_html_attr(tag, &attr.name) {
                continue;
            }

            if attr.value.is_empty() {
                // Boolean valueless attribute: `<div flex items-center p-4 hover:bg-slate-800>`
                let key = format!("bool\x00{}", attr.name);
                if attr_seen.insert(key) {
                    attr_candidates.push(AttributifyEntry {
                        attr_name: attr.name.clone(),
                        value_token: None,
                    });
                }
            } else if is_grouped_attributify_attr(tag, &attr.name) {
                let mut tokens: Vec<String> = Vec::new();
                if attr.value.starts_with('{') && attr.value.ends_with('}') {
                    let expr = if config.variant_group {
                        expand_variant_groups_in_expr(&attr.value)
                    } else {
                        attr.value.clone()
                    };
                    let mut raw_seen = HashSet::new();
                    extract_quoted_tokens_from_expr(&expr, &mut tokens, &mut raw_seen);
                } else if config.variant_group {
                    for t in split_group_tokens(&attr.value) {
                        tokens.extend(expand_single_token(t, 0));
                    }
                } else {
                    tokens = attr.value.split_whitespace().map(|s| s.to_string()).collect();
                }

                for tok in tokens {
                    let key = format!("grp\x00{}\x00{}", attr.name, tok);
                    if attr_seen.insert(key) {
                        attr_candidates.push(AttributifyEntry {
                            attr_name: attr.name.clone(),
                            value_token: Some(tok),
                        });
                    }
                }
            }
        }

        if node.node_type == NodeType::DynamicText && node.content.contains("class") {
            push_expr_class_candidates(
                &node.content,
                config.variant_group,
                class_candidates,
                class_seen,
            );
        }

        collect_attributify_candidates(
            &node.children,
            config,
            class_candidates,
            class_seen,
            attr_candidates,
            attr_seen,
        );
        collect_attributify_candidates(
            &node.consequent,
            config,
            class_candidates,
            class_seen,
            attr_candidates,
            attr_seen,
        );
        collect_attributify_candidates(
            &node.alternate,
            config,
            class_candidates,
            class_seen,
            attr_candidates,
            attr_seen,
        );
    }
}

fn expand_shortcut_leaf_tokens<'a>(
    name: &str,
    shortcuts: &'a HashMap<String, String>,
    outer_variants: &[String],
    outer_important: bool,
    visited: &mut HashSet<String>,
    depth: usize,
    out: &mut Vec<(Vec<String>, bool, String)>,
) {
    if depth >= 8 || !visited.insert(name.to_string()) {
        return;
    }

    if let Some(def) = shortcuts.get(name) {
        let expanded = expand_variant_groups(def);
        for tok in expanded.split_whitespace() {
            let parts = split_variants_outside_brackets(tok);
            let Some((inner_imp, inner_core)) = parse_important_and_core(&parts) else {
                continue;
            };
            let (clean_inner, _) = strip_unocss_prefix(inner_core);

            let mut combined_vars: Vec<String> = outer_variants.to_vec();
            for &v in &parts[..parts.len() - 1] {
                combined_vars.push(v.to_string());
            }
            let combined_imp = outer_important || inner_imp;

            if shortcuts.contains_key(clean_inner) {
                expand_shortcut_leaf_tokens(
                    clean_inner,
                    shortcuts,
                    &combined_vars,
                    combined_imp,
                    visited,
                    depth + 1,
                    out,
                );
            } else {
                out.push((combined_vars, combined_imp, clean_inner.to_string()));
            }
        }
    }

    visited.remove(name);
}

fn resolve_theme_lookup(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("colors.") {
        let token = rest.replace('.', "-");
        return resolve_color(&token);
    }
    if let Some(rest) = path.strip_prefix("spacing.") {
        return resolve_spacing(rest);
    }
    if let Some(rest) = path.strip_prefix("borderRadius.") {
        return resolve_radius(rest).map(|s| s.to_string());
    }
    if let Some(rest) = path.strip_prefix("breakpoints.") {
        return match rest {
            "sm" => Some("640px".to_string()),
            "md" => Some("768px".to_string()),
            "lg" => Some("1024px".to_string()),
            "xl" => Some("1280px".to_string()),
            "2xl" => Some("1536px".to_string()),
            _ => None,
        };
    }
    None
}

fn replace_theme_calls(line: &str) -> (String, bool) {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    let mut changed = false;

    while let Some(idx) = rest.find("theme(") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + "theme(".len()..];
        if let Some(end_paren) = after.find(')') {
            let arg = after[..end_paren]
                .trim()
                .trim_matches('\'')
                .trim_matches('"');
            if let Some(val) = resolve_theme_lookup(arg) {
                out.push_str(&val);
                changed = true;
                rest = &after[end_paren + 1..];
                continue;
            }
        }
        out.push_str("theme(");
        rest = after;
    }
    out.push_str(rest);
    (out, changed)
}

fn expand_apply_tokens(
    tokens_str: &str,
    indent: &str,
    shortcuts: &HashMap<String, String>,
    keyframes: &mut HashSet<&'static str>,
) -> (String, bool) {
    let mut out = String::new();
    let mut used_uno = false;
    let expanded = expand_variant_groups(tokens_str);
    if expanded != tokens_str {
        used_uno = true;
    }

    for tok in expanded.split_whitespace() {
        let (clean, had_un) = strip_unocss_prefix(tok.trim_end_matches('!'));
        let has_bang = tok.ends_with('!') || tok.starts_with('!');
        let clean = clean.strip_prefix('!').unwrap_or(clean);
        if had_un || tok.ends_with('!') {
            used_uno = true;
        }

        if shortcuts.contains_key(clean) {
            used_uno = true;
            let mut leaves = Vec::new();
            let mut visited = HashSet::new();
            expand_shortcut_leaf_tokens(clean, shortcuts, &[], has_bang, &mut visited, 0, &mut leaves);
            for (vars, imp, leaf) in leaves {
                if !vars.is_empty() {
                    continue;
                }
                if let Some((resolved, _)) = resolve_unocss_utility(&leaf) {
                    if let Some(kf) = resolved.needs_keyframe {
                        keyframes.insert(kf);
                    }
                    let bang = if imp { " !important" } else { "" };
                    for (p, v) in resolved.decls {
                        out.push_str(&format!("{}{}: {}{};\n", indent, p, v, bang));
                    }
                }
            }
        } else if let Some((resolved, is_uno)) = resolve_unocss_utility(clean) {
            if is_uno {
                used_uno = true;
            }
            if let Some(kf) = resolved.needs_keyframe {
                keyframes.insert(kf);
            }
            let bang = if has_bang { " !important" } else { "" };
            for (p, v) in resolved.decls {
                out.push_str(&format!("{}{}: {}{};\n", indent, p, v, bang));
            }
        }
    }

    (out, used_uno)
}

struct CssDirectiveOutcome {
    processed_css: String,
    had_tw_directives: bool,
    had_uno_directives: bool,
    keyframes: HashSet<&'static str>,
}

fn process_css_directives(
    raw_css: &str,
    shortcuts: &HashMap<String, String>,
) -> CssDirectiveOutcome {
    let mut out = String::new();
    let mut had_tw = false;
    let mut had_uno = false;
    let mut keyframes = HashSet::new();

    for raw_line in raw_css.lines() {
        let (line_with_theme, used_theme) = replace_theme_calls(raw_line);
        if used_theme {
            had_uno = true;
        }

        let trimmed = line_with_theme.trim();
        let indent: String = line_with_theme
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        if matches!(
            trimmed,
            "@tailwind base;"
                | "@tailwind components;"
                | "@tailwind utilities;"
                | "@import \"tailwindcss\";"
                | "@import 'tailwindcss';"
        ) {
            had_tw = true;
            continue;
        }

        if matches!(
            trimmed,
            "@unocss;"
                | "@unocss preflights;"
                | "@unocss default;"
                | "@unocss all;"
                | "@unocss safelist;"
                | "@import \"unocss\";"
                | "@import 'unocss';"
                | "@import \"@unocss/reset/tailwind.css\";"
                | "@import '@unocss/reset/tailwind.css';"
        ) {
            had_uno = true;
            continue;
        }

        if let Some(after_screen) = trimmed.strip_prefix("@screen ") {
            let mut parts = after_screen.splitn(2, '{');
            let bp = parts.next().unwrap_or("").trim();
            let rest = parts.next().unwrap_or("");
            if let Some(mq) = lt_breakpoint_query(bp).or_else(|| breakpoint_weight_and_query(bp).map(|(_, q)| q)) {
                had_uno = true;
                out.push_str(&format!("{}{} {{{}\n", indent, mq, rest));
                continue;
            }
        }

        if let Some(after_at_apply) = trimmed.strip_prefix("--at-apply:") {
            had_uno = true;
            let tokens_str = after_at_apply.trim_end_matches(';').trim();
            let (expanded_decls, _) =
                expand_apply_tokens(tokens_str, &indent, shortcuts, &mut keyframes);
            out.push_str(&expanded_decls);
            continue;
        }

        if let Some(after_apply) = trimmed.strip_prefix("@apply ") {
            let tokens_str = after_apply.trim_end_matches(';').trim();
            let (expanded_decls, used_uno) =
                expand_apply_tokens(tokens_str, &indent, shortcuts, &mut keyframes);
            if used_uno {
                had_uno = true;
            } else {
                had_tw = true;
            }
            out.push_str(&expanded_decls);
            continue;
        }

        // Single-line rule containing inline `--at-apply:` or `@apply `
        if let Some(idx) = trimmed.find("--at-apply:") {
            let prefix = &trimmed[..idx];
            let rest = &trimmed[idx + "--at-apply:".len()..];
            if let Some(semi) = rest.find(';') {
                let tokens_str = rest[..semi].trim();
                let suffix = &rest[semi + 1..];
                let (expanded_decls, _) =
                    expand_apply_tokens(tokens_str, "", shortcuts, &mut keyframes);
                let inline_decls = expanded_decls.replace('\n', " ");
                had_uno = true;
                out.push_str(&format!("{}{}{}{}\n", indent, prefix, inline_decls.trim(), suffix));
                continue;
            }
        }

        out.push_str(&line_with_theme);
        out.push('\n');
    }

    CssDirectiveOutcome {
        processed_css: out.trim().to_string(),
        had_tw_directives: had_tw,
        had_uno_directives: had_uno,
        keyframes,
    }
}

fn try_resolve_grouped_utility(attr_clean: &str, core: &str) -> Option<(ResolvedUtility, bool)> {
    if core == "~" {
        return resolve_unocss_utility(attr_clean).map(|(r, _)| (r, true));
    }

    let mut attempts: Vec<String> = Vec::with_capacity(5);
    if let Some(after_neg) = core.strip_prefix('-') {
        attempts.push(format!("-{}-{}", attr_clean, after_neg));
        attempts.push(format!("-{}{}", attr_clean, after_neg));
    } else {
        attempts.push(format!("{}-{}", attr_clean, core));
        if matches!(attr_clean, "p" | "m" | "b") {
            attempts.push(format!("{}{}", attr_clean, core));
        }
    }

    if matches!(attr_clean, "flex" | "grid")
        && (core.starts_with("items-")
            || core.starts_with("justify-")
            || core.starts_with("content-")
            || core.starts_with("self-")
            || core.starts_with("place-")
            || core.starts_with("gap-"))
    {
        attempts.push(core.to_string());
    }
    if attr_clean == "border" && core.starts_with("rounded") {
        attempts.push(core.to_string());
    }

    for candidate in attempts {
        if let Some((res, _)) = resolve_unocss_utility(&candidate) {
            return Some((res, true));
        }
    }

    None
}

fn escape_attr_value_word(val: &str) -> String {
    val.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_external_unocss_cli(dir: &Path, comp: &Component) -> Result<String, String> {
    let tmp_dir = dir.join(".zvelte/uno_tmp");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("failed to prepare unocss temp dir: {}", e))?;

    let out_path = tmp_dir.join("uno.css");
    let content_path = dir.join("app.zvelte");
    let _ = comp;

    let output = Command::new("unocss")
        .args([
            &content_path.to_string_lossy(),
            "-o",
            &out_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("failed to run 'unocss' CLI (mode='cli'): {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "unocss CLI exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated = std::fs::read_to_string(&out_path).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(generated)
}

/// Unified Tailwind CSS + UnoCSS compilation pass for a parsed Zvelte component.
pub fn compile_all_styles(
    comp: &Component,
    tw_cfg: &TailwindConfig,
    uno_cfg: &UnoCssConfig,
    project_dir: Option<&Path>,
) -> Result<UnifiedCssOutput, String> {
    // External UnoCSS CLI mode when explicitly configured
    if uno_cfg.enabled && uno_cfg.mode == "cli" {
        let shortcuts: HashMap<String, String> = uno_cfg.shortcuts.iter().cloned().collect();
        let css_dir = process_css_directives(&comp.styles, &shortcuts);
        if let Some(dir) = project_dir {
            let cli_css = run_external_unocss_cli(dir, comp)?;
            return Ok(UnifiedCssOutput {
                processed_custom_style: css_dir.processed_css,
                base_css: String::new(),
                utilities_css: cli_css,
                tailwind_utilities: 0,
                unocss_rules: 1,
            });
        }
    }

    // Delegate strictly to external tailwindcss CLI if requested and UnoCSS isn't active
    if !uno_cfg.explicit && tw_cfg.enabled && tw_cfg.mode == "cli" {
        let tw = compile_tailwind(comp, tw_cfg, project_dir)?;
        return Ok(UnifiedCssOutput {
            processed_custom_style: tw.processed_custom_style,
            base_css: tw.tailwind_base,
            utilities_css: tw.tailwind_utilities,
            tailwind_utilities: tw.utility_count,
            unocss_rules: 0,
        });
    }

    let tw_active = tw_cfg.enabled && tw_cfg.mode != "off";
    let uno_active = uno_cfg.enabled && uno_cfg.mode != "off";

    let shortcuts: HashMap<String, String> = if uno_active {
        uno_cfg.shortcuts.iter().cloned().collect()
    } else {
        HashMap::new()
    };

    let css_dir = process_css_directives(&comp.styles, &shortcuts);

    if !tw_active && !uno_active {
        return Ok(UnifiedCssOutput {
            processed_custom_style: css_dir.processed_css,
            base_css: String::new(),
            utilities_css: String::new(),
            tailwind_utilities: 0,
            unocss_rules: 0,
        });
    }

    let dark_mode = if uno_cfg.explicit {
        uno_cfg.dark_mode.as_str()
    } else if tw_cfg.explicit {
        tw_cfg.dark_mode.as_str()
    } else {
        "media"
    };

    let mut class_candidates = Vec::new();
    let mut class_seen = HashSet::new();
    let mut attr_candidates = Vec::new();
    let mut attr_seen = HashSet::new();

    collect_attributify_candidates(
        &comp.root_nodes,
        uno_cfg,
        &mut class_candidates,
        &mut class_seen,
        &mut attr_candidates,
        &mut attr_seen,
    );

    // Bucket order: layer (0=shortcuts, 1=class utilities, 2=attributify), media weight, media stack
    let mut by_bucket: BTreeMap<(u8, u8, Vec<String>), Vec<String>> = BTreeMap::new();
    let mut rule_seen: HashSet<((u8, u8, Vec<String>), String)> = HashSet::new();
    let mut keyframes = css_dir.keyframes;
    let mut tailwind_utilities = 0usize;
    let mut unocss_rules = 0usize;

    let mut push_rule = |layer: u8,
                         weight: u8,
                         mqs: Vec<String>,
                         rule: String,
                         kf: Option<&'static str>,
                         is_uno: bool| {
        let key = (layer, weight, mqs);
        if rule_seen.insert((key.clone(), rule.clone())) {
            by_bucket.entry(key).or_default().push(rule);
            if is_uno {
                unocss_rules += 1;
            } else {
                tailwind_utilities += 1;
            }
        }
        if let Some(k) = kf {
            keyframes.insert(k);
        }
    };

    // 1. Class utilities & class shortcuts
    for meta in class_candidates {
        let parts = split_variants_outside_brackets(&meta.token);
        let Some((important, core)) = parse_important_and_core(&parts) else {
            continue;
        };
        let (clean_core, had_un) = strip_unocss_prefix(core);
        let outer_vars = &parts[..parts.len() - 1];
        let selector_base = format!(".{}", escape_css_ident(&meta.token));

        if uno_active && shortcuts.contains_key(clean_core) {
            let mut leaves = Vec::new();
            let mut visited = HashSet::new();
            let init_vars: Vec<String> = outer_vars.iter().map(|s| s.to_string()).collect();
            expand_shortcut_leaf_tokens(
                clean_core,
                &shortcuts,
                &init_vars,
                important,
                &mut visited,
                0,
                &mut leaves,
            );
            for (leaf_vars, leaf_imp, leaf_util) in leaves {
                if let Some((resolved, _)) = resolve_unocss_utility(&leaf_util) {
                    let var_refs: Vec<&str> = leaf_vars.iter().map(|s| s.as_str()).collect();
                    if let Some((w, mqs, rule, kf, _)) = compile_rule_with_all_variants(
                        &selector_base,
                        &var_refs,
                        leaf_imp,
                        resolved,
                        dark_mode,
                    ) {
                        push_rule(0, w, mqs, rule, kf, true);
                    }
                }
            }
            continue;
        }

        if let Some((resolved, is_uno_util)) = if uno_active {
            resolve_unocss_utility(clean_core)
        } else {
            resolve_utility(core).map(|r| (r, false))
        } {
            if let Some((w, mqs, rule, kf, used_uno_variant)) = compile_rule_with_all_variants(
                &selector_base,
                outer_vars,
                important,
                resolved,
                dark_mode,
            ) {
                let is_uno = is_uno_util
                    || had_un
                    || used_uno_variant
                    || meta.from_variant_group
                    || meta.token.ends_with('!');
                push_rule(1, w, mqs, rule, kf, is_uno);
            }
        }
    }

    // 2. UnoCSS Attributify Mode
    if uno_active && uno_cfg.attributify {
        for entry in attr_candidates {
            if let Some(val_tok) = entry.value_token {
                // Grouped: `<div bg="slate-900 hover:slate-800">` or `<button hover="bg-sky-500 text-white">`
                let parts = split_variants_outside_brackets(&val_tok);
                let Some((important, core)) = parse_important_and_core(&parts) else {
                    continue;
                };
                let (clean_core, _) = strip_unocss_prefix(core);
                let (clean_attr, _) = strip_unocss_prefix(&entry.attr_name);
                let selector_base = format!(
                    "[{}~=\"{}\"]",
                    escape_css_ident(&entry.attr_name),
                    escape_attr_value_word(&val_tok)
                );

                if VARIANT_PREFIXES.contains(&clean_attr) {
                    let mut all_vars: Vec<&str> = Vec::with_capacity(parts.len());
                    all_vars.push(clean_attr);
                    all_vars.extend_from_slice(&parts[..parts.len() - 1]);

                    if shortcuts.contains_key(clean_core) {
                        let mut leaves = Vec::new();
                        let mut visited = HashSet::new();
                        let init_vars: Vec<String> = all_vars.iter().map(|s| s.to_string()).collect();
                        expand_shortcut_leaf_tokens(
                            clean_core,
                            &shortcuts,
                            &init_vars,
                            important,
                            &mut visited,
                            0,
                            &mut leaves,
                        );
                        for (leaf_vars, leaf_imp, leaf_util) in leaves {
                            if let Some((resolved, _)) = resolve_unocss_utility(&leaf_util) {
                                let var_refs: Vec<&str> =
                                    leaf_vars.iter().map(|s| s.as_str()).collect();
                                if let Some((w, mqs, rule, kf, _)) = compile_rule_with_all_variants(
                                    &selector_base,
                                    &var_refs,
                                    leaf_imp,
                                    resolved,
                                    dark_mode,
                                ) {
                                    push_rule(2, w, mqs, rule, kf, true);
                                }
                            }
                        }
                    } else if let Some((resolved, _)) = resolve_unocss_utility(clean_core) {
                        if let Some((w, mqs, rule, kf, _)) = compile_rule_with_all_variants(
                            &selector_base,
                            &all_vars,
                            important,
                            resolved,
                            dark_mode,
                        ) {
                            push_rule(2, w, mqs, rule, kf, true);
                        }
                    }
                } else if let Some((resolved, _)) = try_resolve_grouped_utility(clean_attr, clean_core) {
                    let outer_vars = &parts[..parts.len() - 1];
                    if let Some((w, mqs, rule, kf, _)) = compile_rule_with_all_variants(
                        &selector_base,
                        outer_vars,
                        important,
                        resolved,
                        dark_mode,
                    ) {
                        push_rule(2, w, mqs, rule, kf, true);
                    }
                }
            } else {
                // Boolean valueless attribute: `<div flex items-center hover:bg-slate-800>`
                let parts = split_variants_outside_brackets(&entry.attr_name);
                let Some((important, core)) = parse_important_and_core(&parts) else {
                    continue;
                };
                let (clean_core, _) = strip_unocss_prefix(core);
                let outer_vars = &parts[..parts.len() - 1];
                let selector_base = format!("[{}]", escape_css_ident(&entry.attr_name));

                if shortcuts.contains_key(clean_core) {
                    let mut leaves = Vec::new();
                    let mut visited = HashSet::new();
                    let init_vars: Vec<String> = outer_vars.iter().map(|s| s.to_string()).collect();
                    expand_shortcut_leaf_tokens(
                        clean_core,
                        &shortcuts,
                        &init_vars,
                        important,
                        &mut visited,
                        0,
                        &mut leaves,
                    );
                    for (leaf_vars, leaf_imp, leaf_util) in leaves {
                        if let Some((resolved, _)) = resolve_unocss_utility(&leaf_util) {
                            let var_refs: Vec<&str> = leaf_vars.iter().map(|s| s.as_str()).collect();
                            if let Some((w, mqs, rule, kf, _)) = compile_rule_with_all_variants(
                                &selector_base,
                                &var_refs,
                                leaf_imp,
                                resolved,
                                dark_mode,
                            ) {
                                push_rule(2, w, mqs, rule, kf, true);
                            }
                        }
                    }
                } else if let Some((resolved, _)) = resolve_unocss_utility(clean_core) {
                    if let Some((w, mqs, rule, kf, _)) = compile_rule_with_all_variants(
                        &selector_base,
                        outer_vars,
                        important,
                        resolved,
                        dark_mode,
                    ) {
                        push_rule(2, w, mqs, rule, kf, true);
                    }
                }
            }
        }
    }

    let preflight_enabled = if uno_cfg.explicit {
        uno_cfg.preflight
    } else if tw_cfg.explicit {
        tw_cfg.preflight
    } else {
        tw_cfg.preflight && uno_cfg.preflight
    };

    let any_active_usage = tailwind_utilities > 0
        || unocss_rules > 0
        || css_dir.had_tw_directives
        || css_dir.had_uno_directives
        || tw_cfg.explicit
        || uno_cfg.explicit;

    let base_css = if preflight_enabled && any_active_usage {
        PREFLIGHT_CSS.to_string()
    } else {
        String::new()
    };

    let mut util_css = String::new();
    if !keyframes.is_empty() {
        let mut sorted_kf: Vec<_> = keyframes.into_iter().collect();
        sorted_kf.sort();
        for kf in sorted_kf {
            let css = keyframe_css(kf);
            if !css.is_empty() {
                util_css.push_str(css);
                util_css.push('\n');
            }
        }
    }

    for ((_, _, mqs), rules) in by_bucket {
        let indent = if mqs.is_empty() { "" } else { "  " };
        for mq in &mqs {
            util_css.push_str(mq);
            util_css.push_str(" {\n");
        }
        for rule in rules {
            util_css.push_str(indent);
            util_css.push_str(&rule);
            util_css.push('\n');
        }
        for _ in &mqs {
            util_css.push_str("}\n");
        }
    }

    Ok(UnifiedCssOutput {
        processed_custom_style: css_dir.processed_css,
        base_css,
        utilities_css: util_css.trim_end().to_string(),
        tailwind_utilities,
        unocss_rules,
    })
}
