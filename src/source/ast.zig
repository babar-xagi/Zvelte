const std = @import("std");
const Allocator = std.mem.Allocator;

pub const ScriptLang = enum {
    python,
    zig,
    js,

    pub fn fromString(str: []const u8) ?ScriptLang {
        if (std.mem.eql(u8, str, "python") or std.mem.eql(u8, str, "py")) return .python;
        if (std.mem.eql(u8, str, "zig")) return .zig;
        if (std.mem.eql(u8, str, "js") or std.mem.eql(u8, str, "javascript")) return .js;
        return null;
    }
};

pub const ScriptContext = enum {
    server,
    native,
    wasm,
    client,

    pub fn fromString(str: []const u8) ?ScriptContext {
        if (std.mem.eql(u8, str, "server")) return .server;
        if (std.mem.eql(u8, str, "native")) return .native;
        if (std.mem.eql(u8, str, "wasm")) return .wasm;
        if (std.mem.eql(u8, str, "client")) return .client;
        return null;
    }
};

pub const ScriptBlock = struct {
    lang: ScriptLang,
    context: ScriptContext,
    code: []const u8,
    line: usize,
    col: usize,
};

pub const StateVar = struct {
    name: []const u8,
    type_name: []const u8,
    initial_value: []const u8,
    line: usize,
};

pub const Attribute = struct {
    name: []const u8,
    value: []const u8,
};

pub const EventBinding = struct {
    event_name: []const u8, // "click", "input"
    expression: []const u8, // "count++", "count = 0"
    line: usize,
};

pub const Binding = struct {
    property: []const u8, // "value", "checked"
    variable: []const u8, // state variable name
    line: usize = 0,
};

pub const NodeType = enum {
    element,
    text,
    interpolation,
    if_block,
    each_block,
};

pub const Node = struct {
    node_type: NodeType,
    tag: []const u8 = "",
    attributes: []const Attribute = &.{},
    events: []const EventBinding = &.{},
    bindings: []const Binding = &.{},
    children: []const Node = &.{},
    else_children: []const Node = &.{},
    content: []const u8 = "", // text, expression, condition, or collection
    item_name: []const u8 = "", // loop variable in each_block
    key_name: []const u8 = "",  // optional key expression in each_block
    line: usize = 0,
};

pub const Component = struct {
    arena: *std.heap.ArenaAllocator,
    scripts: []const ScriptBlock,
    states: []const StateVar,
    root_nodes: []const Node,
    styles: []const u8,

    pub fn deinit(self: *Component, child_allocator: Allocator) void {
        self.arena.deinit();
        child_allocator.destroy(self.arena);
    }
};
