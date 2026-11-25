use super::types::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TomlError {
    MissingSection(String),
    MissingField(String),
    InvalidValue(String),
}

impl std::fmt::Display for TomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TomlError::MissingSection(s) => write!(f, "missing section '[{}]'", s),
            TomlError::MissingField(k) => write!(f, "missing field '{}'", k),
            TomlError::InvalidValue(v) => write!(f, "invalid value '{}'", v),
        }
    }
}

impl std::error::Error for TomlError {}

fn strip_toml_comment(val: &str) -> &str {
    let mut in_double = false;
    let mut in_single = false;
    let mut prev_backslash = false;

    for (idx, ch) in val.char_indices() {
        if in_double {
            if prev_backslash {
                prev_backslash = false;
            } else if ch == '\\' {
                prev_backslash = true;
            } else if ch == '"' {
                in_double = false;
            }
        } else if in_single {
            if ch == '\'' {
                in_single = false;
            }
        } else if ch == '"' {
            in_double = true;
        } else if ch == '\'' {
            in_single = true;
        } else if ch == '#' {
            return val[..idx].trim();
        }
    }
    val.trim()
}

pub fn parse_app_toml(source: &str) -> Result<AppConfig, TomlError> {
    let mut config = AppConfig::default();
    let mut current_section = String::new();

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        // Key = Value
        if let Some(eq_idx) = line.find('=') {
            let key = line[..eq_idx].trim();
            let val_str = strip_toml_comment(line[eq_idx + 1..].trim());

            let val_unquoted = if (val_str.starts_with('"') && val_str.ends_with('"'))
                || (val_str.starts_with('\'') && val_str.ends_with('\''))
            {
                if val_str.len() >= 2 {
                    &val_str[1..val_str.len() - 1]
                } else {
                    val_str
                }
            } else {
                val_str
            };

            match current_section.as_str() {
                "project" => match key {
                    "name" => config.project.name = val_unquoted.to_string(),
                    "version" => config.project.version = val_unquoted.to_string(),
                    "entry" => config.project.entry = val_unquoted.to_string(),
                    _ => {}
                },
                "app" => match key {
                    "target" => config.app.target = val_unquoted.to_string(),
                    "mode" => {
                        config.app.mode = AppMode::from_str(val_unquoted)
                            .ok_or_else(|| TomlError::InvalidValue(val_unquoted.to_string()))?;
                    }
                    "renderer" => {
                        config.app.renderer = Renderer::from_str(val_unquoted)
                            .ok_or_else(|| TomlError::InvalidValue(val_unquoted.to_string()))?;
                    }
                    "strict" => config.app.strict = val_unquoted == "true",
                    _ => {}
                },
                "toolchain" => match key {
                    "rust" | "zig" => config.toolchain.rust = val_unquoted.to_string(),
                    "python" => config.toolchain.python = val_unquoted.to_string(),
                    _ => {}
                },
                "python" => match key {
                    "enabled" => config.python.enabled = val_unquoted == "true",
                    "executable" => config.python.executable = val_unquoted.to_string(),
                    "execution" => config.python.execution = val_unquoted.to_string(),
                    "rust_acceleration" | "zig_acceleration" => {
                        config.python.rust_acceleration = val_unquoted == "true"
                    }
                    _ => {}
                },
                "rust" | "zig" => match key {
                    "enabled" => config.rust.enabled = val_unquoted == "true",
                    "executable" => config.rust.executable = val_unquoted.to_string(),
                    "native_modules" => config.rust.native_modules = val_unquoted == "true",
                    "wasm_modules" => config.rust.wasm_modules = val_unquoted == "true",
                    _ => {}
                },
                "dev" => match key {
                    "host" => config.dev.host = val_unquoted.to_string(),
                    "port" => {
                        config.dev.port = val_unquoted
                            .parse::<u16>()
                            .map_err(|_| TomlError::InvalidValue(val_unquoted.to_string()))?;
                    }
                    "hot_reload" => config.dev.hot_reload = val_unquoted == "true",
                    "open_browser" => config.dev.open_browser = val_unquoted == "true",
                    _ => {}
                },
                "build" => match key {
                    "output" | "out_dir" => config.build.output = val_unquoted.to_string(),
                    "cache" => config.build.cache = val_unquoted.to_string(),
                    "optimization" => config.build.optimization = val_unquoted.to_string(),
                    "minify" => config.build.minify = val_unquoted == "true",
                    "source_maps" => config.build.source_maps = val_unquoted == "true",
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(config)
}
