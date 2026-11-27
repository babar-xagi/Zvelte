#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLang {
    Python,
    Rust,
    Js,
}

impl ScriptLang {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "python" | "py" => Some(ScriptLang::Python),
            "rust" | "rs" => Some(ScriptLang::Rust),
            "js" | "javascript" => Some(ScriptLang::Js),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptLang::Python => "python",
            ScriptLang::Rust => "rust",
            ScriptLang::Js => "js",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptContext {
    Server,
    Native,
    Wasm,
    Client,
}

impl ScriptContext {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "server" => Some(ScriptContext::Server),
            "native" => Some(ScriptContext::Native),
            "wasm" => Some(ScriptContext::Wasm),
            "client" => Some(ScriptContext::Client),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptContext::Server => "server",
            ScriptContext::Native => "native",
            ScriptContext::Wasm => "wasm",
            ScriptContext::Client => "client",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBlock {
    pub lang: ScriptLang,
    pub context: ScriptContext,
    pub code: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVar {
    pub name: String,
    pub type_name: String,
    pub initial_value: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedVar {
    pub name: String,
    pub expression: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBinding {
    pub event_name: String,
    pub expression: String,
    pub modifiers: Vec<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub property: String,
    pub variable: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Element,
    Text,
    DynamicText,
    IfBlock,
    EachBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub node_type: NodeType,
    pub tag: String,
    pub attributes: Vec<Attribute>,
    pub events: Vec<EventBinding>,
    pub bindings: Vec<Binding>,
    pub children: Vec<Node>,
    pub condition: Option<String>,
    pub consequent: Vec<Node>,
    pub alternate: Vec<Node>,
    pub collection: Option<String>,
    pub item_name: Option<String>,
    pub index_name: Option<String>,
    pub key_name: Option<String>,
    pub content: String,
    pub line: usize,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            node_type: NodeType::Element,
            tag: String::new(),
            attributes: Vec::new(),
            events: Vec::new(),
            bindings: Vec::new(),
            children: Vec::new(),
            condition: None,
            consequent: Vec::new(),
            alternate: Vec::new(),
            collection: None,
            item_name: None,
            index_name: None,
            key_name: None,
            content: String::new(),
            line: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Component {
    pub scripts: Vec<ScriptBlock>,
    pub states: Vec<StateVar>,
    pub derived: Vec<DerivedVar>,
    pub root_nodes: Vec<Node>,
    pub styles: String,
}
