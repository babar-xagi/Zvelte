use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::process::Command;

use crate::config::types::TailwindConfig;
use crate::source::ast::{Component, Node, NodeType};

pub struct TailwindOutput {
    pub processed_custom_style: String,
    pub tailwind_base: String,
    pub tailwind_utilities: String,
    pub utility_count: usize,
}

const PREFLIGHT_CSS: &str = r#"@layer base {
  *, ::before, ::after { box-sizing: border-box; border-width: 0; border-style: solid; border-color: #e5e7eb; }
  html { line-height: 1.5; -webkit-text-size-adjust: 100%; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  body { margin: 0; line-height: inherit; }
  h1, h2, h3, h4, h5, h6 { font-size: inherit; font-weight: inherit; }
  a { color: inherit; text-decoration: inherit; }
  button, input, optgroup, select, textarea { font-family: inherit; font-size: 100%; font-weight: inherit; line-height: inherit; color: inherit; margin: 0; padding: 0; }
  button, select { text-transform: none; }
  button, [type='button'], [type='reset'], [type='submit'] { -webkit-appearance: button; background-color: transparent; background-image: none; }
  img, svg, video, canvas { display: block; vertical-align: middle; max-width: 100%; height: auto; }
  ol, ul, menu { list-style: none; margin: 0; padding: 0; }
}"#;

// Standard Tailwind CSS RGB palette (22 color families x 11 shades)
const PALETTE: &[(&str, &[(u16, (u8, u8, u8))])] = &[
    ("slate", &[(50,(248,250,252)),(100,(241,245,249)),(200,(226,232,240)),(300,(203,213,225)),(400,(148,163,184)),(500,(100,116,139)),(600,(71,85,105)),(700,(51,65,85)),(800,(30,41,59)),(900,(15,23,42)),(950,(2,6,23))]),
    ("gray", &[(50,(249,250,251)),(100,(243,244,246)),(200,(229,231,235)),(300,(209,213,219)),(400,(156,163,175)),(500,(107,114,128)),(600,(75,85,99)),(700,(55,65,81)),(800,(31,41,55)),(900,(17,24,39)),(950,(3,7,18))]),
    ("zinc", &[(50,(250,250,250)),(100,(244,244,245)),(200,(228,228,231)),(300,(212,212,216)),(400,(161,161,170)),(500,(113,113,122)),(600,(82,82,91)),(700,(63,63,70)),(800,(39,39,42)),(900,(24,24,27)),(950,(9,9,11))]),
    ("neutral", &[(50,(250,250,250)),(100,(245,245,245)),(200,(229,229,229)),(300,(212,212,212)),(400,(163,163,163)),(500,(115,115,115)),(600,(82,82,82)),(700,(64,64,64)),(800,(38,38,38)),(900,(23,23,23)),(950,(10,10,10))]),
    ("stone", &[(50,(250,250,249)),(100,(245,245,244)),(200,(231,229,228)),(300,(214,211,209)),(400,(168,162,158)),(500,(120,113,108)),(600,(87,83,78)),(700,(68,64,60)),(800,(41,37,36)),(900,(28,25,23)),(950,(12,10,9))]),
    ("red", &[(50,(254,242,242)),(100,(254,226,226)),(200,(254,202,202)),(300,(252,165,165)),(400,(248,113,113)),(500,(239,68,68)),(600,(220,38,38)),(700,(185,28,28)),(800,(153,27,27)),(900,(127,29,29)),(950,(69,10,10))]),
    ("orange", &[(50,(255,247,237)),(100,(255,237,213)),(200,(254,215,170)),(300,(253,186,116)),(400,(251,146,60)),(500,(249,115,22)),(600,(234,88,12)),(700,(194,65,12)),(800,(154,52,18)),(900,(124,45,18)),(950,(67,20,7))]),
    ("amber", &[(50,(255,251,235)),(100,(254,243,199)),(200,(253,230,138)),(300,(252,211,77)),(400,(251,191,36)),(500,(245,158,11)),(600,(217,119,6)),(700,(180,83,9)),(800,(146,64,14)),(900,(120,53,15)),(950,(69,26,3))]),
    ("yellow", &[(50,(254,252,232)),(100,(254,249,195)),(200,(254,240,138)),(300,(253,224,71)),(400,(250,204,21)),(500,(234,179,8)),(600,(202,138,4)),(700,(161,98,7)),(800,(133,77,14)),(900,(113,63,18)),(950,(66,32,6))]),
    ("lime", &[(50,(247,254,231)),(100,(236,252,203)),(200,(217,249,157)),(300,(190,242,100)),(400,(163,230,53)),(500,(132,204,22)),(600,(101,163,13)),(700,(77,124,15)),(800,(63,98,18)),(900,(54,83,20)),(950,(26,46,5))]),
    ("green", &[(50,(240,253,244)),(100,(220,252,231)),(200,(187,247,208)),(300,(134,239,172)),(400,(74,222,128)),(500,(34,197,94)),(600,(22,163,74)),(700,(21,128,61)),(800,(22,101,52)),(900,(20,83,45)),(950,(5,46,22))]),
    ("emerald", &[(50,(236,253,245)),(100,(209,250,229)),(200,(167,243,208)),(300,(110,231,183)),(400,(52,211,153)),(500,(16,185,129)),(600,(5,150,105)),(700,(4,120,87)),(800,(6,95,70)),(900,(6,78,59)),(950,(2,44,34))]),
    ("teal", &[(50,(240,253,250)),(100,(204,251,241)),(200,(153,246,228)),(300,(94,234,212)),(400,(45,212,191)),(500,(20,184,166)),(600,(13,148,136)),(700,(15,118,110)),(800,(17,94,89)),(900,(19,78,74)),(950,(4,47,46))]),
    ("cyan", &[(50,(236,254,255)),(100,(207,250,254)),(200,(165,243,252)),(300,(103,232,249)),(400,(34,211,238)),(500,(6,182,212)),(600,(8,145,178)),(700,(14,116,144)),(800,(21,94,117)),(900,(22,78,99)),(950,(8,51,68))]),
    ("sky", &[(50,(240,249,255)),(100,(224,242,254)),(200,(186,230,253)),(300,(125,211,252)),(400,(56,189,248)),(500,(14,165,233)),(600,(2,132,199)),(700,(3,105,161)),(800,(7,89,133)),(900,(12,74,110)),(950,(8,47,73))]),
    ("blue", &[(50,(239,246,255)),(100,(219,234,254)),(200,(191,219,254)),(300,(147,197,253)),(400,(96,165,250)),(500,(59,130,246)),(600,(37,99,235)),(700,(29,78,216)),(800,(30,64,175)),(900,(30,58,138)),(950,(23,37,84))]),
    ("indigo", &[(50,(238,242,255)),(100,(224,231,255)),(200,(199,210,254)),(300,(165,180,252)),(400,(129,140,248)),(500,(99,102,241)),(600,(79,70,229)),(700,(67,56,202)),(800,(55,48,163)),(900,(49,46,129)),(950,(30,27,75))]),
    ("violet", &[(50,(245,243,255)),(100,(237,233,254)),(200,(221,214,254)),(300,(196,181,253)),(400,(167,139,250)),(500,(139,92,246)),(600,(124,58,237)),(700,(109,40,217)),(800,(91,33,182)),(900,(76,29,149)),(950,(46,16,101))]),
    ("purple", &[(50,(250,245,255)),(100,(243,232,255)),(200,(233,213,255)),(300,(216,180,254)),(400,(192,132,252)),(500,(168,85,247)),(600,(147,51,234)),(700,(126,34,206)),(800,(107,33,168)),(900,(88,28,135)),(950,(59,7,100))]),
    ("fuchsia", &[(50,(253,244,255)),(100,(250,232,255)),(200,(245,208,254)),(300,(240,171,252)),(400,(232,121,249)),(500,(217,70,239)),(600,(192,38,211)),(700,(162,28,175)),(800,(134,25,143)),(900,(112,26,117)),(950,(74,4,78))]),
    ("pink", &[(50,(253,242,248)),(100,(252,231,243)),(200,(251,207,232)),(300,(249,168,212)),(400,(244,114,182)),(500,(236,72,153)),(600,(219,39,119)),(700,(190,24,93)),(800,(157,23,77)),(900,(131,24,67)),(950,(80,7,36))]),
    ("rose", &[(50,(255,241,242)),(100,(255,228,230)),(200,(254,205,211)),(300,(253,164,175)),(400,(251,113,133)),(500,(244,63,94)),(600,(225,29,72)),(700,(190,18,60)),(800,(159,18,57)),(900,(136,19,55)),(950,(76,5,25))]),
];

fn parse_arbitrary_val(s: &str) -> Option<String> {
    if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
        Some(s[1..s.len() - 1].replace('_', " "))
    } else {
        None
    }
}

fn parse_alpha(s: &str) -> Option<String> {
    if let Some(arb) = parse_arbitrary_val(s) {
        return Some(arb);
    }
    let num: u32 = s.parse().ok()?;
    if num > 100 {
        return None;
    }
    if num == 100 {
        Some("1".to_string())
    } else if num == 0 {
        Some("0".to_string())
    } else if num % 10 == 0 {
        Some(format!("0.{}", num / 10))
    } else {
        Some(format!("0.{:02}", num))
    }
}

fn resolve_color(token: &str) -> Option<String> {
    let (base, alpha_opt) = if let Some(slash) = token.find('/') {
        (&token[..slash], Some(parse_alpha(&token[slash + 1..])?))
    } else {
        (token, None)
    };

    if let Some(arb) = parse_arbitrary_val(base) {
        return Some(match alpha_opt {
            Some(a) if arb.starts_with('#') && arb.len() == 7 => {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&arb[1..3], 16),
                    u8::from_str_radix(&arb[3..5], 16),
                    u8::from_str_radix(&arb[5..7], 16),
                ) {
                    format!("rgb({} {} {} / {})", r, g, b, a)
                } else {
                    arb
                }
            }
            Some(a) => format!("color-mix(in srgb, {} calc({} * 100%), transparent)", arb, a),
            None => arb,
        });
    }

    match base {
        "transparent" => return Some("transparent".to_string()),
        "current" => return Some("currentColor".to_string()),
        "inherit" => return Some("inherit".to_string()),
        "black" => {
            return Some(match alpha_opt {
                Some(a) => format!("rgb(0 0 0 / {})", a),
                None => "#000000".to_string(),
            });
        }
        "white" => {
            return Some(match alpha_opt {
                Some(a) => format!("rgb(255 255 255 / {})", a),
                None => "#ffffff".to_string(),
            });
        }
        _ => {}
    }

    let dash = base.rfind('-')?;
    let family = &base[..dash];
    let shade: u16 = base[dash + 1..].parse().ok()?;

    for (name, shades) in PALETTE {
        if *name == family {
            for (sh, (r, g, b)) in *shades {
                if *sh == shade {
                    return Some(match alpha_opt {
                        Some(a) => format!("rgb({} {} {} / {})", r, g, b, a),
                        None => format!("rgb({} {} {})", r, g, b),
                    });
                }
            }
        }
    }
    None
}

fn resolve_spacing(token: &str) -> Option<String> {
    if let Some(arb) = parse_arbitrary_val(token) {
        return Some(arb);
    }
    match token {
        "0" => return Some("0px".to_string()),
        "px" => return Some("1px".to_string()),
        "auto" => return Some("auto".to_string()),
        "full" => return Some("100%".to_string()),
        _ => {}
    }
    let val: f32 = token.parse().ok()?;
    let valid_stop = matches!(
        token,
        "0.5" | "1" | "1.5" | "2" | "2.5" | "3" | "3.5" | "4" | "5" | "6" | "7" | "8" | "9"
            | "10" | "11" | "12" | "14" | "16" | "20" | "24" | "28" | "32" | "36" | "40"
            | "44" | "48" | "52" | "56" | "60" | "64" | "72" | "80" | "96"
    );
    if !valid_stop {
        return None;
    }
    let rem = val * 0.25;
    if rem.fract() == 0.0 {
        Some(format!("{}rem", rem as i32))
    } else {
        Some(format!("{}rem", rem))
    }
}

fn resolve_fraction(token: &str) -> Option<String> {
    let slash = token.find('/')?;
    let num: f64 = token[..slash].parse().ok()?;
    let den: f64 = token[slash + 1..].parse().ok()?;
    if den == 0.0 || num <= 0.0 || num > den {
        return None;
    }
    let pct = (num / den) * 100.0;
    if (pct - pct.round()).abs() < 1e-6 {
        Some(format!("{}%", pct.round() as i64))
    } else {
        Some(format!("{:.6}%", pct))
    }
}

fn resolve_size(token: &str) -> Option<String> {
    if let Some(s) = resolve_spacing(token) {
        return Some(s);
    }
    if let Some(f) = resolve_fraction(token) {
        return Some(f);
    }
    match token {
        "screen" => Some("100%".to_string()),
        "svw" => Some("100svw".to_string()),
        "svh" => Some("100svh".to_string()),
        "dvw" => Some("100dvw".to_string()),
        "dvh" => Some("100dvh".to_string()),
        "min" => Some("min-content".to_string()),
        "max" => Some("max-content".to_string()),
        "fit" => Some("fit-content".to_string()),
        _ => None,
    }
}

#[derive(Default)]
struct ResolvedUtility {
    decls: Vec<(String, String)>,
    child_combinator: Option<&'static str>,
    needs_keyframe: Option<&'static str>,
}

impl ResolvedUtility {
    fn one(prop: &str, val: impl Into<String>) -> Self {
        Self {
            decls: vec![(prop.to_string(), val.into())],
            ..Default::default()
        }
    }

    fn many(pairs: Vec<(&str, String)>) -> Self {
        Self {
            decls: pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            ..Default::default()
        }
    }
}

fn resolve_radius(size: &str) -> Option<&'static str> {
    match size {
        "" => Some("0.25rem"),
        "none" => Some("0px"),
        "sm" => Some("0.125rem"),
        "md" => Some("0.375rem"),
        "lg" => Some("0.5rem"),
        "xl" => Some("0.75rem"),
        "2xl" => Some("1rem"),
        "3xl" => Some("1.5rem"),
        "full" => Some("9999px"),
        _ => None,
    }
}

fn resolve_utility(raw_utility: &str) -> Option<ResolvedUtility> {
    // Handle universal arbitrary property: [prop:value]
    if let Some(arb) = parse_arbitrary_val(raw_utility) {
        if let Some(colon) = arb.find(':') {
            let prop = arb[..colon].trim();
            let val = arb[colon + 1..].trim();
            if !prop.is_empty() && !val.is_empty() {
                return Some(ResolvedUtility::one(prop, val));
            }
        }
        return None;
    }

    let (neg, u) = if let Some(rest) = raw_utility.strip_prefix('-') {
        (true, rest)
    } else {
        (false, raw_utility)
    };

    let apply_neg = |v: String| -> String {
        if neg && v != "0px" && v != "auto" {
            format!("-{}", v)
        } else {
            v
        }
    };

    // Exact keyword utilities (only when non-negative unless noted)
    if !neg {
        if let Some(r) = match u {
            // Display & Layout
            "block" => Some(ResolvedUtility::one("display", "block")),
            "inline-block" => Some(ResolvedUtility::one("display", "inline-block")),
            "inline" => Some(ResolvedUtility::one("display", "inline")),
            "flex" => Some(ResolvedUtility::one("display", "flex")),
            "inline-flex" => Some(ResolvedUtility::one("display", "inline-flex")),
            "table" => Some(ResolvedUtility::one("display", "table")),
            "grid" => Some(ResolvedUtility::one("display", "grid")),
            "inline-grid" => Some(ResolvedUtility::one("display", "inline-grid")),
            "contents" => Some(ResolvedUtility::one("display", "contents")),
            "hidden" => Some(ResolvedUtility::one("display", "none")),
            "visible" => Some(ResolvedUtility::one("visibility", "visible")),
            "invisible" => Some(ResolvedUtility::one("visibility", "hidden")),
            "collapse" => Some(ResolvedUtility::one("visibility", "collapse")),
            "static" => Some(ResolvedUtility::one("position", "static")),
            "fixed" => Some(ResolvedUtility::one("position", "fixed")),
            "absolute" => Some(ResolvedUtility::one("position", "absolute")),
            "relative" => Some(ResolvedUtility::one("position", "relative")),
            "sticky" => Some(ResolvedUtility::one("position", "sticky")),
            "isolate" => Some(ResolvedUtility::one("isolation", "isolate")),
            "box-border" => Some(ResolvedUtility::one("box-sizing", "border-box")),
            "box-content" => Some(ResolvedUtility::one("box-sizing", "content-box")),
            // Overflow
            "overflow-auto" => Some(ResolvedUtility::one("overflow", "auto")),
            "overflow-hidden" => Some(ResolvedUtility::one("overflow", "hidden")),
            "overflow-clip" => Some(ResolvedUtility::one("overflow", "clip")),
            "overflow-visible" => Some(ResolvedUtility::one("overflow", "visible")),
            "overflow-scroll" => Some(ResolvedUtility::one("overflow", "scroll")),
            "overflow-x-auto" => Some(ResolvedUtility::one("overflow-x", "auto")),
            "overflow-x-hidden" => Some(ResolvedUtility::one("overflow-x", "hidden")),
            "overflow-y-auto" => Some(ResolvedUtility::one("overflow-y", "auto")),
            "overflow-x-scroll" => Some(ResolvedUtility::one("overflow-x", "scroll")),
            "overflow-y-hidden" => Some(ResolvedUtility::one("overflow-y", "hidden")),
            "overflow-y-scroll" => Some(ResolvedUtility::one("overflow-y", "scroll")),
            // Flex / Grid
            "flex-row" => Some(ResolvedUtility::one("flex-direction", "row")),
            "flex-row-reverse" => Some(ResolvedUtility::one("flex-direction", "row-reverse")),
            "flex-col" => Some(ResolvedUtility::one("flex-direction", "column")),
            "flex-col-reverse" => Some(ResolvedUtility::one("flex-direction", "column-reverse")),
            "flex-wrap" => Some(ResolvedUtility::one("flex-wrap", "wrap")),
            "flex-wrap-reverse" => Some(ResolvedUtility::one("flex-wrap", "wrap-reverse")),
            "flex-nowrap" => Some(ResolvedUtility::one("flex-wrap", "nowrap")),
            "flex-1" => Some(ResolvedUtility::one("flex", "1 1 0%")),
            "flex-auto" => Some(ResolvedUtility::one("flex", "1 1 auto")),
            "flex-initial" => Some(ResolvedUtility::one("flex", "0 1 auto")),
            "flex-none" => Some(ResolvedUtility::one("flex", "none")),
            "grow" | "flex-grow" => Some(ResolvedUtility::one("flex-grow", "1")),
            "grow-0" | "flex-grow-0" => Some(ResolvedUtility::one("flex-grow", "0")),
            "shrink" | "flex-shrink" => Some(ResolvedUtility::one("flex-shrink", "1")),
            "shrink-0" | "flex-shrink-0" => Some(ResolvedUtility::one("flex-shrink", "0")),
            "order-first" => Some(ResolvedUtility::one("order", "-9999")),
            "order-last" => Some(ResolvedUtility::one("order", "9999")),
            "order-none" => Some(ResolvedUtility::one("order", "0")),
            "justify-start" => Some(ResolvedUtility::one("justify-content", "flex-start")),
            "justify-end" => Some(ResolvedUtility::one("justify-content", "flex-end")),
            "justify-center" => Some(ResolvedUtility::one("justify-content", "center")),
            "justify-between" => Some(ResolvedUtility::one("justify-content", "space-between")),
            "justify-around" => Some(ResolvedUtility::one("justify-content", "space-around")),
            "justify-evenly" => Some(ResolvedUtility::one("justify-content", "space-evenly")),
            "items-start" => Some(ResolvedUtility::one("align-items", "flex-start")),
            "items-end" => Some(ResolvedUtility::one("align-items", "flex-end")),
            "items-center" => Some(ResolvedUtility::one("align-items", "center")),
            "items-baseline" => Some(ResolvedUtility::one("align-items", "baseline")),
            "items-stretch" => Some(ResolvedUtility::one("align-items", "stretch")),
            "self-auto" => Some(ResolvedUtility::one("align-self", "auto")),
            "self-start" => Some(ResolvedUtility::one("align-self", "flex-start")),
            "self-end" => Some(ResolvedUtility::one("align-self", "flex-end")),
            "self-center" => Some(ResolvedUtility::one("align-self", "center")),
            "self-stretch" => Some(ResolvedUtility::one("align-self", "stretch")),
            "place-items-center" => Some(ResolvedUtility::one("place-items", "center")),
            "place-content-center" => Some(ResolvedUtility::one("place-content", "center")),
            "grid-flow-row" => Some(ResolvedUtility::one("grid-auto-flow", "row")),
            "grid-flow-col" => Some(ResolvedUtility::one("grid-auto-flow", "column")),
            // Typography
            "font-sans" => Some(ResolvedUtility::one("font-family", "ui-sans-serif, system-ui, -apple-system, sans-serif")),
            "font-serif" => Some(ResolvedUtility::one("font-family", "ui-serif, Georgia, Cambria, serif")),
            "font-mono" => Some(ResolvedUtility::one("font-family", "ui-monospace, SFMono-Regular, Menlo, monospace")),
            "font-thin" => Some(ResolvedUtility::one("font-weight", "100")),
            "font-extralight" => Some(ResolvedUtility::one("font-weight", "200")),
            "font-light" => Some(ResolvedUtility::one("font-weight", "300")),
            "font-normal" => Some(ResolvedUtility::one("font-weight", "400")),
            "font-medium" => Some(ResolvedUtility::one("font-weight", "500")),
            "font-semibold" => Some(ResolvedUtility::one("font-weight", "600")),
            "font-bold" => Some(ResolvedUtility::one("font-weight", "700")),
            "font-extrabold" => Some(ResolvedUtility::one("font-weight", "800")),
            "font-black" => Some(ResolvedUtility::one("font-weight", "900")),
            "italic" => Some(ResolvedUtility::one("font-style", "italic")),
            "not-italic" => Some(ResolvedUtility::one("font-style", "normal")),
            "antialiased" => Some(ResolvedUtility::many(vec![("-webkit-font-smoothing", "antialiased".into()), ("-moz-osx-font-smoothing", "grayscale".into())])),
            "text-xs" => Some(ResolvedUtility::many(vec![("font-size", "0.75rem".into()), ("line-height", "1rem".into())])),
            "text-sm" => Some(ResolvedUtility::many(vec![("font-size", "0.875rem".into()), ("line-height", "1.25rem".into())])),
            "text-base" => Some(ResolvedUtility::many(vec![("font-size", "1rem".into()), ("line-height", "1.5rem".into())])),
            "text-lg" => Some(ResolvedUtility::many(vec![("font-size", "1.125rem".into()), ("line-height", "1.75rem".into())])),
            "text-xl" => Some(ResolvedUtility::many(vec![("font-size", "1.25rem".into()), ("line-height", "1.75rem".into())])),
            "text-2xl" => Some(ResolvedUtility::many(vec![("font-size", "1.5rem".into()), ("line-height", "2rem".into())])),
            "text-3xl" => Some(ResolvedUtility::many(vec![("font-size", "1.875rem".into()), ("line-height", "2.25rem".into())])),
            "text-4xl" => Some(ResolvedUtility::many(vec![("font-size", "2.25rem".into()), ("line-height", "2.5rem".into())])),
            "text-5xl" => Some(ResolvedUtility::many(vec![("font-size", "3rem".into()), ("line-height", "1".into())])),
            "text-6xl" => Some(ResolvedUtility::many(vec![("font-size", "3.75rem".into()), ("line-height", "1".into())])),
            "text-left" => Some(ResolvedUtility::one("text-align", "left")),
            "text-center" => Some(ResolvedUtility::one("text-align", "center")),
            "text-right" => Some(ResolvedUtility::one("text-align", "right")),
            "text-justify" => Some(ResolvedUtility::one("text-align", "justify")),
            "text-start" => Some(ResolvedUtility::one("text-align", "start")),
            "text-end" => Some(ResolvedUtility::one("text-align", "end")),
            "underline" => Some(ResolvedUtility::one("text-decoration-line", "underline")),
            "overline" => Some(ResolvedUtility::one("text-decoration-line", "overline")),
            "line-through" => Some(ResolvedUtility::one("text-decoration-line", "line-through")),
            "no-underline" => Some(ResolvedUtility::one("text-decoration-line", "none")),
            "uppercase" => Some(ResolvedUtility::one("text-transform", "uppercase")),
            "lowercase" => Some(ResolvedUtility::one("text-transform", "lowercase")),
            "capitalize" => Some(ResolvedUtility::one("text-transform", "capitalize")),
            "normal-case" => Some(ResolvedUtility::one("text-transform", "none")),
            "truncate" => Some(ResolvedUtility::many(vec![("overflow", "hidden".into()), ("text-overflow", "ellipsis".into()), ("white-space", "nowrap".into())])),
            "whitespace-nowrap" => Some(ResolvedUtility::one("white-space", "nowrap")),
            "whitespace-pre" => Some(ResolvedUtility::one("white-space", "pre")),
            "whitespace-pre-line" => Some(ResolvedUtility::one("white-space", "pre-line")),
            "whitespace-pre-wrap" => Some(ResolvedUtility::one("white-space", "pre-wrap")),
            "whitespace-normal" => Some(ResolvedUtility::one("white-space", "normal")),
            "text-wrap" => Some(ResolvedUtility::one("text-wrap", "wrap")),
            "text-nowrap" => Some(ResolvedUtility::one("text-wrap", "nowrap")),
            "text-balance" => Some(ResolvedUtility::one("text-wrap", "balance")),
            "text-ellipsis" => Some(ResolvedUtility::one("text-overflow", "ellipsis")),
            "text-clip" => Some(ResolvedUtility::one("text-overflow", "clip")),
            "line-clamp-1" => Some(ResolvedUtility::many(vec![("overflow", "hidden".into()), ("display", "-webkit-box".into()), ("-webkit-box-orient", "vertical".into()), ("-webkit-line-clamp", "1".into())])),
            "line-clamp-2" => Some(ResolvedUtility::many(vec![("overflow", "hidden".into()), ("display", "-webkit-box".into()), ("-webkit-box-orient", "vertical".into()), ("-webkit-line-clamp", "2".into())])),
            "line-clamp-3" => Some(ResolvedUtility::many(vec![("overflow", "hidden".into()), ("display", "-webkit-box".into()), ("-webkit-box-orient", "vertical".into()), ("-webkit-line-clamp", "3".into())])),
            "line-clamp-none" => Some(ResolvedUtility::many(vec![("overflow", "visible".into()), ("display", "block".into()), ("-webkit-line-clamp", "none".into())])),
            "break-words" => Some(ResolvedUtility::one("overflow-wrap", "break-word")),
            "break-all" => Some(ResolvedUtility::one("word-break", "break-all")),
            "tracking-tighter" => Some(ResolvedUtility::one("letter-spacing", "-0.05em")),
            "tracking-tight" => Some(ResolvedUtility::one("letter-spacing", "-0.025em")),
            "tracking-normal" => Some(ResolvedUtility::one("letter-spacing", "0em")),
            "tracking-wide" => Some(ResolvedUtility::one("letter-spacing", "0.025em")),
            "tracking-wider" => Some(ResolvedUtility::one("letter-spacing", "0.05em")),
            "tracking-widest" => Some(ResolvedUtility::one("letter-spacing", "0.1em")),
            "leading-none" => Some(ResolvedUtility::one("line-height", "1")),
            "leading-tight" => Some(ResolvedUtility::one("line-height", "1.25")),
            "leading-snug" => Some(ResolvedUtility::one("line-height", "1.375")),
            "leading-normal" => Some(ResolvedUtility::one("line-height", "1.5")),
            "leading-relaxed" => Some(ResolvedUtility::one("line-height", "1.625")),
            "leading-loose" => Some(ResolvedUtility::one("line-height", "2")),
            // Borders & Radius
            "rounded" => Some(ResolvedUtility::one("border-radius", "0.25rem")),
            "rounded-none" => Some(ResolvedUtility::one("border-radius", "0px")),
            "rounded-sm" => Some(ResolvedUtility::one("border-radius", "0.125rem")),
            "rounded-md" => Some(ResolvedUtility::one("border-radius", "0.375rem")),
            "rounded-lg" => Some(ResolvedUtility::one("border-radius", "0.5rem")),
            "rounded-xl" => Some(ResolvedUtility::one("border-radius", "0.75rem")),
            "rounded-2xl" => Some(ResolvedUtility::one("border-radius", "1rem")),
            "rounded-3xl" => Some(ResolvedUtility::one("border-radius", "1.5rem")),
            "rounded-full" => Some(ResolvedUtility::one("border-radius", "9999px")),
            "border" => Some(ResolvedUtility::one("border-width", "1px")),
            "border-0" => Some(ResolvedUtility::one("border-width", "0px")),
            "border-2" => Some(ResolvedUtility::one("border-width", "2px")),
            "border-4" => Some(ResolvedUtility::one("border-width", "4px")),
            "border-8" => Some(ResolvedUtility::one("border-width", "8px")),
            "border-t" => Some(ResolvedUtility::one("border-top-width", "1px")),
            "border-r" => Some(ResolvedUtility::one("border-right-width", "1px")),
            "border-b" => Some(ResolvedUtility::one("border-bottom-width", "1px")),
            "border-l" => Some(ResolvedUtility::one("border-left-width", "1px")),
            "border-x" => Some(ResolvedUtility::many(vec![("border-left-width", "1px".into()), ("border-right-width", "1px".into())])),
            "border-y" => Some(ResolvedUtility::many(vec![("border-top-width", "1px".into()), ("border-bottom-width", "1px".into())])),
            "border-solid" => Some(ResolvedUtility::one("border-style", "solid")),
            "border-dashed" => Some(ResolvedUtility::one("border-style", "dashed")),
            "border-dotted" => Some(ResolvedUtility::one("border-style", "dotted")),
            "border-none" => Some(ResolvedUtility::one("border-style", "none")),
            "outline-none" => Some(ResolvedUtility::many(vec![("outline", "2px solid transparent".into()), ("outline-offset", "2px".into())])),
            "outline" => Some(ResolvedUtility::one("outline-style", "solid")),
            "ring" => Some(ResolvedUtility::one("box-shadow", "0 0 0 3px var(--tw-ring-color, rgb(59 130 246 / 0.5))")),
            "ring-0" => Some(ResolvedUtility::one("box-shadow", "0 0 0 0px var(--tw-ring-color, rgb(59 130 246 / 0.5))")),
            "ring-1" => Some(ResolvedUtility::one("box-shadow", "0 0 0 1px var(--tw-ring-color, rgb(59 130 246 / 0.5))")),
            "ring-2" => Some(ResolvedUtility::one("box-shadow", "0 0 0 2px var(--tw-ring-color, rgb(59 130 246 / 0.5))")),
            "ring-4" => Some(ResolvedUtility::one("box-shadow", "0 0 0 4px var(--tw-ring-color, rgb(59 130 246 / 0.5))")),
            // Effects & Filters
            "shadow-sm" => Some(ResolvedUtility::one("box-shadow", "0 1px 2px 0 rgb(0 0 0 / 0.05)")),
            "shadow" => Some(ResolvedUtility::one("box-shadow", "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)")),
            "shadow-md" => Some(ResolvedUtility::one("box-shadow", "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)")),
            "shadow-lg" => Some(ResolvedUtility::one("box-shadow", "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)")),
            "shadow-xl" => Some(ResolvedUtility::one("box-shadow", "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)")),
            "shadow-2xl" => Some(ResolvedUtility::one("box-shadow", "0 25px 50px -12px rgb(0 0 0 / 0.25)")),
            "shadow-inner" => Some(ResolvedUtility::one("box-shadow", "inset 0 2px 4px 0 rgb(0 0 0 / 0.05)")),
            "shadow-none" => Some(ResolvedUtility::one("box-shadow", "0 0 #0000")),
            "backdrop-blur-none" => Some(ResolvedUtility::one("backdrop-filter", "blur(0)")),
            "backdrop-blur-sm" => Some(ResolvedUtility::one("backdrop-filter", "blur(4px)")),
            "backdrop-blur" => Some(ResolvedUtility::one("backdrop-filter", "blur(8px)")),
            "backdrop-blur-md" => Some(ResolvedUtility::one("backdrop-filter", "blur(12px)")),
            "backdrop-blur-lg" => Some(ResolvedUtility::one("backdrop-filter", "blur(16px)")),
            "backdrop-blur-xl" => Some(ResolvedUtility::one("backdrop-filter", "blur(24px)")),
            "backdrop-blur-2xl" => Some(ResolvedUtility::one("backdrop-filter", "blur(40px)")),
            "blur-sm" => Some(ResolvedUtility::one("filter", "blur(4px)")),
            "blur" => Some(ResolvedUtility::one("filter", "blur(8px)")),
            "blur-lg" => Some(ResolvedUtility::one("filter", "blur(16px)")),
            // Gradients & Clip
            "bg-clip-text" => Some(ResolvedUtility::many(vec![("-webkit-background-clip", "text".into()), ("background-clip", "text".into())])),
            "bg-gradient-to-r" => Some(ResolvedUtility::one("background-image", "linear-gradient(to right, var(--tw-gradient-stops))")),
            "bg-gradient-to-l" => Some(ResolvedUtility::one("background-image", "linear-gradient(to left, var(--tw-gradient-stops))")),
            "bg-gradient-to-t" => Some(ResolvedUtility::one("background-image", "linear-gradient(to top, var(--tw-gradient-stops))")),
            "bg-gradient-to-b" => Some(ResolvedUtility::one("background-image", "linear-gradient(to bottom, var(--tw-gradient-stops))")),
            "bg-gradient-to-tr" => Some(ResolvedUtility::one("background-image", "linear-gradient(to top right, var(--tw-gradient-stops))")),
            "bg-gradient-to-tl" => Some(ResolvedUtility::one("background-image", "linear-gradient(to top left, var(--tw-gradient-stops))")),
            "bg-gradient-to-br" => Some(ResolvedUtility::one("background-image", "linear-gradient(to bottom right, var(--tw-gradient-stops))")),
            "bg-gradient-to-bl" => Some(ResolvedUtility::one("background-image", "linear-gradient(to bottom left, var(--tw-gradient-stops))")),
            // Transitions & Animation
            "transition" => Some(ResolvedUtility::many(vec![
                ("transition-property", "color, background-color, border-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform, filter, backdrop-filter".into()),
                ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)".into()),
                ("transition-duration", "150ms".into()),
            ])),
            "transition-all" => Some(ResolvedUtility::many(vec![("transition-property", "all".into()), ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)".into()), ("transition-duration", "150ms".into())])),
            "transition-colors" => Some(ResolvedUtility::many(vec![("transition-property", "color, background-color, border-color, text-decoration-color, fill, stroke".into()), ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)".into()), ("transition-duration", "150ms".into())])),
            "transition-opacity" => Some(ResolvedUtility::many(vec![("transition-property", "opacity".into()), ("transition-duration", "150ms".into())])),
            "transition-transform" => Some(ResolvedUtility::many(vec![("transition-property", "transform".into()), ("transition-duration", "150ms".into())])),
            "transition-none" => Some(ResolvedUtility::one("transition-property", "none")),
            "ease-linear" => Some(ResolvedUtility::one("transition-timing-function", "linear")),
            "ease-in" => Some(ResolvedUtility::one("transition-timing-function", "cubic-bezier(0.4, 0, 1, 1)")),
            "ease-out" => Some(ResolvedUtility::one("transition-timing-function", "cubic-bezier(0, 0, 0.2, 1)")),
            "ease-in-out" => Some(ResolvedUtility::one("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)")),
            "animate-none" => Some(ResolvedUtility::one("animation", "none")),
            "animate-spin" => Some(ResolvedUtility { decls: vec![("animation".into(), "spin 1s linear infinite".into())], needs_keyframe: Some("spin"), ..Default::default() }),
            "animate-ping" => Some(ResolvedUtility { decls: vec![("animation".into(), "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite".into())], needs_keyframe: Some("ping"), ..Default::default() }),
            "animate-pulse" => Some(ResolvedUtility { decls: vec![("animation".into(), "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite".into())], needs_keyframe: Some("pulse"), ..Default::default() }),
            "animate-bounce" => Some(ResolvedUtility { decls: vec![("animation".into(), "bounce 1s infinite".into())], needs_keyframe: Some("bounce"), ..Default::default() }),
            // Interactivity & Aspect & Object
            "cursor-pointer" => Some(ResolvedUtility::one("cursor", "pointer")),
            "cursor-default" => Some(ResolvedUtility::one("cursor", "default")),
            "cursor-not-allowed" => Some(ResolvedUtility::one("cursor", "not-allowed")),
            "cursor-wait" => Some(ResolvedUtility::one("cursor", "wait")),
            "select-none" => Some(ResolvedUtility::one("user-select", "none")),
            "select-text" => Some(ResolvedUtility::one("user-select", "text")),
            "select-all" => Some(ResolvedUtility::one("user-select", "all")),
            "pointer-events-none" => Some(ResolvedUtility::one("pointer-events", "none")),
            "pointer-events-auto" => Some(ResolvedUtility::one("pointer-events", "auto")),
            "appearance-none" => Some(ResolvedUtility::one("appearance", "none")),
            "sr-only" => Some(ResolvedUtility::many(vec![
                ("position", "absolute".into()), ("width", "1px".into()), ("height", "1px".into()),
                ("padding", "0".into()), ("margin", "-1px".into()), ("overflow", "hidden".into()),
                ("clip", "rect(0, 0, 0, 0)".into()), ("white-space", "nowrap".into()), ("border-width", "0".into()),
            ])),
            "aspect-auto" => Some(ResolvedUtility::one("aspect-ratio", "auto")),
            "aspect-square" => Some(ResolvedUtility::one("aspect-ratio", "1 / 1")),
            "aspect-video" => Some(ResolvedUtility::one("aspect-ratio", "16 / 9")),
            "object-contain" => Some(ResolvedUtility::one("object-fit", "contain")),
            "object-cover" => Some(ResolvedUtility::one("object-fit", "cover")),
            "object-fill" => Some(ResolvedUtility::one("object-fit", "fill")),
            _ => None,
        } {
            return Some(r);
        }
    }

    // Dynamic prefix utilities
    if let Some(rest) = u.strip_prefix("p-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding", v));
    }
    if let Some(rest) = u.strip_prefix("px-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::many(vec![("padding-left", v.clone()), ("padding-right", v)]));
    }
    if let Some(rest) = u.strip_prefix("py-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::many(vec![("padding-top", v.clone()), ("padding-bottom", v)]));
    }
    if let Some(rest) = u.strip_prefix("pt-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding-top", v));
    }
    if let Some(rest) = u.strip_prefix("pr-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding-right", v));
    }
    if let Some(rest) = u.strip_prefix("pb-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding-bottom", v));
    }
    if let Some(rest) = u.strip_prefix("ps-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding-inline-start", v));
    }
    if let Some(rest) = u.strip_prefix("pe-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding-inline-end", v));
    }
    if let Some(rest) = u.strip_prefix("pl-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("padding-left", v));
    }

    if let Some(rest) = u.strip_prefix("m-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("mx-") {
        return resolve_spacing(rest).map(|v| {
            let v = apply_neg(v);
            ResolvedUtility::many(vec![("margin-left", v.clone()), ("margin-right", v)])
        });
    }
    if let Some(rest) = u.strip_prefix("my-") {
        return resolve_spacing(rest).map(|v| {
            let v = apply_neg(v);
            ResolvedUtility::many(vec![("margin-top", v.clone()), ("margin-bottom", v)])
        });
    }
    if let Some(rest) = u.strip_prefix("mt-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin-top", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("mr-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin-right", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("mb-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin-bottom", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("ms-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin-inline-start", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("me-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin-inline-end", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("ml-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("margin-left", apply_neg(v)));
    }

    if let Some(rest) = u.strip_prefix("gap-x-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("column-gap", v));
    }
    if let Some(rest) = u.strip_prefix("gap-y-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("row-gap", v));
    }
    if let Some(rest) = u.strip_prefix("gap-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility::one("gap", v));
    }
    if let Some(rest) = u.strip_prefix("space-x-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility {
            decls: vec![("margin-left".into(), apply_neg(v))],
            child_combinator: Some(" > :not([hidden]) ~ :not([hidden])"),
            ..Default::default()
        });
    }
    if let Some(rest) = u.strip_prefix("space-y-") {
        return resolve_spacing(rest).map(|v| ResolvedUtility {
            decls: vec![("margin-top".into(), apply_neg(v))],
            child_combinator: Some(" > :not([hidden]) ~ :not([hidden])"),
            ..Default::default()
        });
    }

    // Sizing
    if let Some(rest) = u.strip_prefix("w-") {
        return match rest {
            "screen" => Some(ResolvedUtility::one("width", "100vw")),
            _ => resolve_size(rest).map(|v| ResolvedUtility::one("width", v)),
        };
    }
    if let Some(rest) = u.strip_prefix("h-") {
        return match rest {
            "screen" => Some(ResolvedUtility::one("height", "100vh")),
            _ => resolve_size(rest).map(|v| ResolvedUtility::one("height", v)),
        };
    }
    if let Some(rest) = u.strip_prefix("size-") {
        return resolve_size(rest).map(|v| ResolvedUtility::many(vec![("width", v.clone()), ("height", v)]));
    }
    if let Some(rest) = u.strip_prefix("min-w-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("min-width", v));
    }
    if let Some(rest) = u.strip_prefix("min-h-") {
        return match rest {
            "screen" => Some(ResolvedUtility::one("min-height", "100vh")),
            _ => resolve_size(rest).map(|v| ResolvedUtility::one("min-height", v)),
        };
    }
    if let Some(rest) = u.strip_prefix("max-w-") {
        let val = match rest {
            "none" => Some("none".to_string()),
            "xs" => Some("20rem".to_string()),
            "sm" => Some("24rem".to_string()),
            "md" => Some("28rem".to_string()),
            "lg" => Some("32rem".to_string()),
            "xl" => Some("36rem".to_string()),
            "2xl" => Some("42rem".to_string()),
            "3xl" => Some("48rem".to_string()),
            "4xl" => Some("56rem".to_string()),
            "5xl" => Some("64rem".to_string()),
            "6xl" => Some("72rem".to_string()),
            "7xl" => Some("80rem".to_string()),
            "prose" => Some("65ch".to_string()),
            "screen-sm" => Some("640px".to_string()),
            "screen-md" => Some("768px".to_string()),
            "screen-lg" => Some("1024px".to_string()),
            "screen-xl" => Some("1280px".to_string()),
            "screen-2xl" => Some("1536px".to_string()),
            _ => resolve_size(rest),
        };
        return val.map(|v| ResolvedUtility::one("max-width", v));
    }
    if let Some(rest) = u.strip_prefix("max-h-") {
        return match rest {
            "screen" => Some(ResolvedUtility::one("max-height", "100vh")),
            "none" => Some(ResolvedUtility::one("max-height", "none")),
            _ => resolve_size(rest).map(|v| ResolvedUtility::one("max-height", v)),
        };
    }

    // Positioning & z-index
    if let Some(rest) = u.strip_prefix("inset-x-") {
        return resolve_size(rest).map(|v| {
            let v = apply_neg(v);
            ResolvedUtility::many(vec![("left", v.clone()), ("right", v)])
        });
    }
    if let Some(rest) = u.strip_prefix("inset-y-") {
        return resolve_size(rest).map(|v| {
            let v = apply_neg(v);
            ResolvedUtility::many(vec![("top", v.clone()), ("bottom", v)])
        });
    }
    if let Some(rest) = u.strip_prefix("inset-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("inset", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("top-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("top", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("right-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("right", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("bottom-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("bottom", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("left-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("left", apply_neg(v)));
    }
    if let Some(rest) = u.strip_prefix("z-") {
        if rest == "auto" {
            return Some(ResolvedUtility::one("z-index", "auto"));
        }
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("z-index", arb));
        }
        if let Ok(n) = rest.parse::<i32>() {
            return Some(ResolvedUtility::one("z-index", apply_neg(n.to_string())));
        }
    }

    // Grid columns / spans / rows
    if let Some(rest) = u.strip_prefix("grid-cols-") {
        if rest == "none" {
            return Some(ResolvedUtility::one("grid-template-columns", "none"));
        }
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("grid-template-columns", arb));
        }
        if let Ok(n) = rest.parse::<u32>() {
            if n > 0 && n <= 12 {
                return Some(ResolvedUtility::one(
                    "grid-template-columns",
                    format!("repeat({}, minmax(0, 1fr))", n),
                ));
            }
        }
    }
    if let Some(rest) = u.strip_prefix("grid-rows-") {
        if rest == "none" {
            return Some(ResolvedUtility::one("grid-template-rows", "none"));
        }
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("grid-template-rows", arb));
        }
        if let Ok(n) = rest.parse::<u32>() {
            if n > 0 && n <= 12 {
                return Some(ResolvedUtility::one(
                    "grid-template-rows",
                    format!("repeat({}, minmax(0, 1fr))", n),
                ));
            }
        }
    }
    if let Some(rest) = u.strip_prefix("col-span-") {
        if rest == "full" {
            return Some(ResolvedUtility::one("grid-column", "1 / -1"));
        }
        if let Ok(n) = rest.parse::<u32>() {
            return Some(ResolvedUtility::one("grid-column", format!("span {} / span {}", n, n)));
        }
    }
    if let Some(rest) = u.strip_prefix("row-span-") {
        if rest == "full" {
            return Some(ResolvedUtility::one("grid-row", "1 / -1"));
        }
        if let Ok(n) = rest.parse::<u32>() {
            return Some(ResolvedUtility::one("grid-row", format!("span {} / span {}", n, n)));
        }
    }

    // Colors: bg-, text-, border-, ring-, from-, via-, to-, fill-, stroke-, placeholder-
    if let Some(rest) = u.strip_prefix("bg-") {
        return resolve_color(rest).map(|c| ResolvedUtility::one("background-color", c));
    }
    if let Some(rest) = u.strip_prefix("text-") {
        if let Some(arb) = parse_arbitrary_val(rest) {
            if arb.ends_with("px") || arb.ends_with("rem") || arb.ends_with("em") {
                return Some(ResolvedUtility::one("font-size", arb));
            }
        }
        return resolve_color(rest).map(|c| ResolvedUtility::one("color", c));
    }
    for (side, css_side) in [("t-", "top"), ("r-", "right"), ("b-", "bottom"), ("l-", "left")] {
        let prefix = format!("border-{}", side);
        if let Some(rest) = u.strip_prefix(&prefix) {
            if let Some(w) = match rest {
                "0" => Some("0px".to_string()),
                "2" => Some("2px".to_string()),
                "4" => Some("4px".to_string()),
                "8" => Some("8px".to_string()),
                _ => parse_arbitrary_val(rest),
            } {
                return Some(ResolvedUtility::one(&format!("border-{}-width", css_side), w));
            }
            if let Some(color) = resolve_color(rest) {
                return Some(ResolvedUtility::one(&format!("border-{}-color", css_side), color));
            }
        }
    }
    if let Some(rest) = u.strip_prefix("border-") {
        if let Some(color) = resolve_color(rest) {
            return Some(ResolvedUtility::one("border-color", color));
        }
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("border-width", arb));
        }
    }
    if let Some(rest) = u.strip_prefix("ring-") {
        if let Some(color) = resolve_color(rest) {
            return Some(ResolvedUtility::one("--tw-ring-color", color));
        }
    }
    if let Some(rest) = u.strip_prefix("from-") {
        return resolve_color(rest).map(|c| {
            ResolvedUtility::many(vec![
                ("--tw-gradient-from", c),
                ("--tw-gradient-stops", "var(--tw-gradient-from), var(--tw-gradient-to, rgb(255 255 255 / 0))".to_string()),
            ])
        });
    }
    if let Some(rest) = u.strip_prefix("via-") {
        return resolve_color(rest).map(|c| {
            ResolvedUtility::one(
                "--tw-gradient-stops",
                format!("var(--tw-gradient-from), {}, var(--tw-gradient-to, rgb(255 255 255 / 0))", c),
            )
        });
    }
    if let Some(rest) = u.strip_prefix("to-") {
        return resolve_color(rest).map(|c| ResolvedUtility::one("--tw-gradient-to", c));
    }
    if let Some(rest) = u.strip_prefix("fill-") {
        return resolve_color(rest).map(|c| ResolvedUtility::one("fill", c));
    }
    if let Some(rest) = u.strip_prefix("stroke-") {
        if let Some(color) = resolve_color(rest) {
            return Some(ResolvedUtility::one("stroke", color));
        }
        if let Ok(n) = rest.parse::<u32>() {
            return Some(ResolvedUtility::one("stroke-width", n.to_string()));
        }
    }
    if let Some(rest) = u.strip_prefix("rounded-t-") {
        let r = resolve_radius(rest).map(|s| s.to_string()).or_else(|| parse_arbitrary_val(rest));
        return r.map(|v| ResolvedUtility::many(vec![("border-top-left-radius", v.clone()), ("border-top-right-radius", v)]));
    }
    if let Some(rest) = u.strip_prefix("rounded-b-") {
        let r = resolve_radius(rest).map(|s| s.to_string()).or_else(|| parse_arbitrary_val(rest));
        return r.map(|v| ResolvedUtility::many(vec![("border-bottom-left-radius", v.clone()), ("border-bottom-right-radius", v)]));
    }
    if let Some(rest) = u.strip_prefix("rounded-l-") {
        let r = resolve_radius(rest).map(|s| s.to_string()).or_else(|| parse_arbitrary_val(rest));
        return r.map(|v| ResolvedUtility::many(vec![("border-top-left-radius", v.clone()), ("border-bottom-left-radius", v)]));
    }
    if let Some(rest) = u.strip_prefix("rounded-r-") {
        let r = resolve_radius(rest).map(|s| s.to_string()).or_else(|| parse_arbitrary_val(rest));
        return r.map(|v| ResolvedUtility::many(vec![("border-top-right-radius", v.clone()), ("border-bottom-right-radius", v)]));
    }
    if let Some(rest) = u.strip_prefix("rounded-") {
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("border-radius", arb));
        }
        if let Some(v) = resolve_radius(rest) {
            return Some(ResolvedUtility::one("border-radius", v));
        }
    }
    if let Some(rest) = u.strip_prefix("content-") {
        if rest == "none" {
            return Some(ResolvedUtility::one("content", "none"));
        }
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("content", arb));
        }
    }
    if let Some(rest) = u.strip_prefix("opacity-") {
        return parse_alpha(rest).map(|a| ResolvedUtility::one("opacity", a));
    }
    if let Some(rest) = u.strip_prefix("duration-") {
        if let Some(arb) = parse_arbitrary_val(rest) {
            return Some(ResolvedUtility::one("transition-duration", arb));
        }
        if let Ok(ms) = rest.parse::<u32>() {
            return Some(ResolvedUtility::one("transition-duration", format!("{}ms", ms)));
        }
    }
    if let Some(rest) = u.strip_prefix("delay-") {
        if let Ok(ms) = rest.parse::<u32>() {
            return Some(ResolvedUtility::one("transition-delay", format!("{}ms", ms)));
        }
    }

    // Transforms
    if let Some(rest) = u.strip_prefix("scale-") {
        if let Ok(n) = rest.parse::<f32>() {
            return Some(ResolvedUtility::one("transform", format!("scale({})", n / 100.0)));
        }
    }
    if let Some(rest) = u.strip_prefix("rotate-") {
        if let Ok(deg) = rest.parse::<i32>() {
            return Some(ResolvedUtility::one("transform", format!("rotate({}deg)", if neg { -deg } else { deg })));
        }
    }
    if let Some(rest) = u.strip_prefix("translate-x-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("transform", format!("translateX({})", apply_neg(v))));
    }
    if let Some(rest) = u.strip_prefix("translate-y-") {
        return resolve_size(rest).map(|v| ResolvedUtility::one("transform", format!("translateY({})", apply_neg(v))));
    }

    // Divide helpers
    if u == "divide-x" {
        return Some(ResolvedUtility {
            decls: vec![("border-left-width".into(), "1px".into())],
            child_combinator: Some(" > :not([hidden]) ~ :not([hidden])"),
            ..Default::default()
        });
    }
    if u == "divide-y" {
        return Some(ResolvedUtility {
            decls: vec![("border-top-width".into(), "1px".into())],
            child_combinator: Some(" > :not([hidden]) ~ :not([hidden])"),
            ..Default::default()
        });
    }
    if let Some(rest) = u.strip_prefix("divide-") {
        return resolve_color(rest).map(|c| ResolvedUtility {
            decls: vec![("border-color".into(), c)],
            child_combinator: Some(" > :not([hidden]) ~ :not([hidden])"),
            ..Default::default()
        });
    }

    None
}

fn split_variants_outside_brackets(class_token: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut bracket_depth = 0usize;
    let mut start = 0usize;

    for (idx, c) in class_token.char_indices() {
        match c {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ':' if bracket_depth == 0 => {
                parts.push(&class_token[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(&class_token[start..]);
    parts
}

fn escape_css_ident(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    for c in raw.chars() {
        match c {
            ':' | '/' | '[' | ']' | '.' | '#' | '(' | ')' | '%' | '!' | ',' | '*' | '&' | '>'
            | '\'' | '"' | '+' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn breakpoint_weight_and_query(bp: &str) -> Option<(u8, &'static str)> {
    match bp {
        "sm" => Some((1, "@media (min-width: 640px)")),
        "md" => Some((2, "@media (min-width: 768px)")),
        "lg" => Some((3, "@media (min-width: 1024px)")),
        "xl" => Some((4, "@media (min-width: 1280px)")),
        "2xl" => Some((5, "@media (min-width: 1536px)")),
        "max-sm" => Some((6, "@media not all and (min-width: 640px)")),
        "max-md" => Some( (7, "@media not all and (min-width: 768px)")),
        "max-lg" => Some((8, "@media not all and (min-width: 1024px)")),
        "max-xl" => Some((9, "@media not all and (min-width: 1280px)")),
        "max-2xl" => Some((10, "@media not all and (min-width: 1536px)")),
        _ => None,
    }
}

fn compile_single_utility_rule(
    full_class: &str,
    config: &TailwindConfig,
) -> Option<(u8, Vec<String>, String, Option<&'static str>)> {
    let parts = split_variants_outside_brackets(full_class);
    if parts.is_empty() {
        return None;
    }

    let raw_util = *parts.last()?;
    let (important, clean_util) = if let Some(u) = raw_util.strip_prefix('!') {
        (true, u)
    } else if let Some(first) = parts.first().filter(|p| p.starts_with('!') && parts.len() == 1) {
        (true, &first[1..])
    } else {
        (false, raw_util)
    };

    let mut resolved = resolve_utility(clean_util)?;
    let mut media_queries: Vec<String> = Vec::new();
    let mut weight = 0u8;
    let mut prefix_sel = String::new();
    let mut suffix_pseudo = String::new();

    for &variant in &parts[..parts.len() - 1] {
        if let Some((w, mq)) = breakpoint_weight_and_query(variant) {
            weight = weight.max(w);
            media_queries.push(mq.to_string());
        } else if variant == "dark" {
            if config.dark_mode == "class" {
                prefix_sel.push_str(".dark ");
            } else {
                media_queries.push("@media (prefers-color-scheme: dark)".to_string());
            }
        } else if let Some(arb_var) = parse_arbitrary_val(variant) {
            if let Some(sub) = arb_var.strip_prefix('&') {
                suffix_pseudo.push_str(sub);
            } else {
                return None;
            }
        } else {
            match variant {
                "hover" => suffix_pseudo.push_str(":hover"),
                "focus" => suffix_pseudo.push_str(":focus"),
                "focus-visible" => suffix_pseudo.push_str(":focus-visible"),
                "focus-within" => suffix_pseudo.push_str(":focus-within"),
                "active" => suffix_pseudo.push_str(":active"),
                "visited" => suffix_pseudo.push_str(":visited"),
                "disabled" => suffix_pseudo.push_str(":disabled"),
                "enabled" => suffix_pseudo.push_str(":enabled"),
                "checked" => suffix_pseudo.push_str(":checked"),
                "required" => suffix_pseudo.push_str(":required"),
                "invalid" => suffix_pseudo.push_str(":invalid"),
                "valid" => suffix_pseudo.push_str(":valid"),
                "placeholder-shown" => suffix_pseudo.push_str(":placeholder-shown"),
                "empty" => suffix_pseudo.push_str(":empty"),
                "first" => suffix_pseudo.push_str(":first-child"),
                "last" => suffix_pseudo.push_str(":last-child"),
                "only" => suffix_pseudo.push_str(":only-child"),
                "odd" => suffix_pseudo.push_str(":nth-child(odd)"),
                "even" => suffix_pseudo.push_str(":nth-child(even)"),
                "open" => suffix_pseudo.push_str("[open]"),
                "group-hover" => prefix_sel.push_str(".group:hover "),
                "group-focus" => prefix_sel.push_str(".group:focus "),
                "group-active" => prefix_sel.push_str(".group:active "),
                "peer-checked" => prefix_sel.push_str(".peer:checked ~ "),
                "peer-focus" => prefix_sel.push_str(".peer:focus ~ "),
                "before" => {
                    suffix_pseudo.push_str("::before");
                    if !resolved.decls.iter().any(|(k, _)| k == "content") {
                        resolved.decls.insert(0, ("content".to_string(), "\"\"".to_string()));
                    }
                }
                "after" => {
                    suffix_pseudo.push_str("::after");
                    if !resolved.decls.iter().any(|(k, _)| k == "content") {
                        resolved.decls.insert(0, ("content".to_string(), "\"\"".to_string()));
                    }
                }
                "placeholder" => suffix_pseudo.push_str("::placeholder"),
                "selection" => suffix_pseudo.push_str("::selection"),
                "file" => suffix_pseudo.push_str("::file-selector-button"),
                "marker" => suffix_pseudo.push_str("::marker"),
                _ => return None,
            }
        }
    }

    let child_comb = resolved.child_combinator.unwrap_or("");
    let escaped_class = escape_css_ident(full_class);
    let selector = format!("{}.{}{}{}", prefix_sel, escaped_class, suffix_pseudo, child_comb);

    let imp = if important { " !important" } else { "" };
    let decls_str: Vec<String> = resolved
        .decls
        .into_iter()
        .map(|(p, v)| format!("{}: {}{};", p, v, imp))
        .collect();

    let rule = format!("{} {{ {} }}", selector, decls_str.join(" "));
    Some((weight, media_queries, rule, resolved.needs_keyframe))
}

fn push_tokens(chunk: &str, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    for tok in chunk.split_whitespace() {
        if seen.insert(tok.to_string()) {
            out.push(tok.to_string());
        }
    }
}

fn extract_quoted_tokens_from_expr(expr: &str, out: &mut Vec<String>, seen: &mut HashSet<String>) {
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            i += 1;
            let mut static_chunk = String::new();
            while i < chars.len() {
                let sc = chars[i];
                if sc == '\\' && i + 1 < chars.len() {
                    i += 1;
                    static_chunk.push(chars[i]);
                    i += 1;
                    continue;
                }
                if sc == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                    push_tokens(&static_chunk, out, seen);
                    static_chunk.clear();
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
                    extract_quoted_tokens_from_expr(&inner_expr, out, seen);
                    continue;
                }
                if sc == '`' {
                    i += 1;
                    break;
                }
                static_chunk.push(sc);
                i += 1;
            }
            push_tokens(&static_chunk, out, seen);
        } else if c == '"' || c == '\'' {
            let q = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() {
                let sc = chars[i];
                if sc == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(chars[i]);
                    i += 1;
                    continue;
                }
                if sc == q {
                    i += 1;
                    break;
                }
                s.push(sc);
                i += 1;
            }
            push_tokens(&s, out, seen);
        } else {
            i += 1;
        }
    }
}

fn walk_nodes_classes(nodes: &[Node], out: &mut Vec<String>, seen: &mut HashSet<String>) {
    for node in nodes {
        for attr in &node.attributes {
            if attr.name == "class" {
                if attr.value.starts_with('{') && attr.value.ends_with('}') {
                    extract_quoted_tokens_from_expr(&attr.value, out, seen);
                } else {
                    for tok in attr.value.split_whitespace() {
                        if seen.insert(tok.to_string()) {
                            out.push(tok.to_string());
                        }
                    }
                }
            }
        }
        if node.node_type == NodeType::DynamicText && node.content.contains("class") {
            extract_quoted_tokens_from_expr(&node.content, out, seen);
        }
        walk_nodes_classes(&node.children, out, seen);
        walk_nodes_classes(&node.consequent, out, seen);
        walk_nodes_classes(&node.alternate, out, seen);
    }
}

pub fn collect_candidate_classes(comp: &Component) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    walk_nodes_classes(&comp.root_nodes, &mut out, &mut seen);
    out
}

fn expand_apply_directives(raw_css: &str) -> (String, bool, HashSet<&'static str>) {
    let mut out = String::new();
    let mut had_tailwind_syntax = false;
    let mut keyframes = HashSet::new();

    for line in raw_css.lines() {
        let trimmed = line.trim();
        if trimmed == "@tailwind base;"
            || trimmed == "@tailwind components;"
            || trimmed == "@tailwind utilities;"
            || trimmed == "@import \"tailwindcss\";"
            || trimmed == "@import 'tailwindcss';"
        {
            had_tailwind_syntax = true;
            continue;
        }

        if let Some(after_apply) = trimmed.strip_prefix("@apply ") {
            had_tailwind_syntax = true;
            let tokens_str = after_apply.trim_end_matches(';').trim();
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            for tok in tokens_str.split_whitespace() {
                if let Some(resolved) = resolve_utility(tok) {
                    if let Some(kf) = resolved.needs_keyframe {
                        keyframes.insert(kf);
                    }
                    for (prop, val) in resolved.decls {
                        out.push_str(&format!("{}{}: {};\n", indent, prop, val));
                    }
                }
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    (out.trim().to_string(), had_tailwind_syntax, keyframes)
}

fn keyframe_css(name: &str) -> &'static str {
    match name {
        "spin" => "@keyframes spin { to { transform: rotate(360deg); } }",
        "ping" => "@keyframes ping { 75%, 100% { transform: scale(2); opacity: 0; } }",
        "pulse" => "@keyframes pulse { 50% { opacity: .5; } }",
        "bounce" => "@keyframes bounce { 0%, 100% { transform: translateY(-25%); animation-timing-function: cubic-bezier(0.8,0,1,1); } 50% { transform: none; animation-timing-function: cubic-bezier(0,0,0.2,1); } }",
        _ => "",
    }
}

pub fn compile_tailwind(
    comp: &Component,
    config: &TailwindConfig,
    project_dir: Option<&Path>,
) -> Result<TailwindOutput, String> {
    let (processed_custom_style, had_tw_directives, apply_keyframes) =
        expand_apply_directives(&comp.styles);

    if !config.enabled || config.mode == "off" {
        return Ok(TailwindOutput {
            processed_custom_style,
            tailwind_base: String::new(),
            tailwind_utilities: String::new(),
            utility_count: 0,
        });
    }

    // Optional external tailwindcss CLI mode when explicitly configured
    if config.mode == "cli" {
        if let Some(dir) = project_dir {
            let cli_css = run_external_tailwind_cli(dir, comp, &processed_custom_style)?;
            return Ok(TailwindOutput {
                processed_custom_style,
                tailwind_base: String::new(),
                tailwind_utilities: cli_css,
                utility_count: 1,
            });
        }
    }

    let candidates = collect_candidate_classes(comp);
    let mut by_bucket: BTreeMap<(u8, Vec<String>), Vec<String>> = BTreeMap::new();
    let mut keyframes = apply_keyframes;
    let mut utility_count = 0usize;

    for cls in candidates {
        if let Some((weight, mqs, rule, kf)) = compile_single_utility_rule(&cls, config) {
            utility_count += 1;
            if let Some(k) = kf {
                keyframes.insert(k);
            }
            by_bucket.entry((weight, mqs)).or_default().push(rule);
        }
    }

    // Only emit Preflight base CSS if Tailwind classes/directives are used OR [tailwind] is explicitly enabled
    let should_emit_base =
        config.preflight && (utility_count > 0 || had_tw_directives || config.explicit);

    let tailwind_base = if should_emit_base {
        PREFLIGHT_CSS.to_string()
    } else {
        String::new()
    };

    let mut util_css = String::new();
    if !keyframes.is_empty() {
        let mut sorted_kf: Vec<_> = keyframes.into_iter().collect();
        sorted_kf.sort();
        for kf in sorted_kf {
            util_css.push_str(keyframe_css(kf));
            util_css.push('\n');
        }
    }

    for ((_, mqs), rules) in by_bucket {
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

    Ok(TailwindOutput {
        processed_custom_style,
        tailwind_base,
        tailwind_utilities: util_css.trim_end().to_string(),
        utility_count,
    })
}

fn run_external_tailwind_cli(
    dir: &Path,
    comp: &Component,
    custom_css: &str,
) -> Result<String, String> {
    let tmp_dir = dir.join(".zvelte/tw_tmp");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("failed to prepare tailwind temp dir: {}", e))?;

    let in_css = format!("@tailwind base;\n@tailwind components;\n@tailwind utilities;\n{}", custom_css);
    let in_path = tmp_dir.join("input.css");
    let out_path = tmp_dir.join("output.css");
    let content_path = dir.join("app.zvelte");
    std::fs::write(&in_path, in_css).map_err(|e| e.to_string())?;

    let _ = comp;
    let status = Command::new("tailwindcss")
        .args([
            "-i",
            &in_path.to_string_lossy(),
            "-o",
            &out_path.to_string_lossy(),
            "--content",
            &content_path.to_string_lossy(),
            "--minify",
        ])
        .output()
        .map_err(|e| format!("failed to run 'tailwindcss' CLI (mode='cli'): {}", e))?;

    if !status.status.success() {
        return Err(format!(
            "tailwindcss CLI exited with error: {}",
            String::from_utf8_lossy(&status.stderr)
        ));
    }

    let generated = std::fs::read_to_string(&out_path).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(generated)
}
