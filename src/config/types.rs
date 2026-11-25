#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Python,
    Rust,
    Hybrid,
}

impl AppMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "python" | "py" => Some(AppMode::Python),
            "rust" | "rs" => Some(AppMode::Rust),
            "hybrid" => Some(AppMode::Hybrid),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AppMode::Python => "python",
            AppMode::Rust => "rust",
            AppMode::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    Js,
    Wasm,
    Auto,
}

impl Renderer {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "js" => Some(Renderer::Js),
            "wasm" => Some(Renderer::Wasm),
            "auto" => Some(Renderer::Auto),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Renderer::Js => "js",
            Renderer::Wasm => "wasm",
            Renderer::Auto => "auto",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub entry: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSection {
    pub target: String,
    pub mode: AppMode,
    pub renderer: Renderer,
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainConfig {
    pub rust: String,
    pub python: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonConfig {
    pub enabled: bool,
    pub executable: String,
    pub execution: String,
    pub rust_acceleration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustConfig {
    pub enabled: bool,
    pub executable: String,
    pub native_modules: bool,
    pub wasm_modules: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevConfig {
    pub host: String,
    pub port: u16,
    pub hot_reload: bool,
    pub open_browser: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildConfig {
    pub output: String,
    pub cache: String,
    pub optimization: String,
    pub minify: bool,
    pub source_maps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub project: ProjectConfig,
    pub app: AppSection,
    pub toolchain: ToolchainConfig,
    pub python: PythonConfig,
    pub rust: RustConfig,
    pub dev: DevConfig,
    pub build: BuildConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "zvelte-app".to_string(),
                version: "0.1.0".to_string(),
                entry: "app.zvelte".to_string(),
            },
            app: AppSection {
                target: "web".to_string(),
                mode: AppMode::Hybrid,
                renderer: Renderer::Js,
                strict: true,
            },
            toolchain: ToolchainConfig {
                rust: "1.80+".to_string(),
                python: "3.10+".to_string(),
            },
            python: PythonConfig {
                enabled: true,
                executable: "python3".to_string(),
                execution: "server".to_string(),
                rust_acceleration: true,
            },
            rust: RustConfig {
                enabled: true,
                executable: "rustc".to_string(),
                native_modules: true,
                wasm_modules: true,
            },
            dev: DevConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                hot_reload: true,
                open_browser: false,
            },
            build: BuildConfig {
                output: "dist".to_string(),
                cache: ".zvelte".to_string(),
                optimization: "release".to_string(),
                minify: false,
                source_maps: true,
            },
        }
    }
}
