const std = @import("std");
const Allocator = std.mem.Allocator;
const ast = @import("ast.zig");

pub const ParseError = error{
    UnexpectedEOF,
    InvalidTag,
    MismatchedTag,
    UnterminatedScript,
    UnterminatedState,
    UnterminatedStyle,
    InvalidStateDeclaration,
    MissingScriptAttribute,
    OutOfMemory,
};

pub const Parser = struct {
    allocator: Allocator,
    source: []const u8,
    pos: usize,
    line: usize,
    col: usize,

    pub fn init(allocator: Allocator, source: []const u8) Parser {
        return .{
            .allocator = allocator,
            .source = source,
            .pos = 0,
            .line = 1,
            .col = 1,
        };
    }

    fn peek(self: *const Parser) ?u8 {
        if (self.pos < self.source.len) return self.source[self.pos];
        return null;
    }

    fn advance(self: *Parser) ?u8 {
        if (self.pos >= self.source.len) return null;
        const c = self.source[self.pos];
        self.pos += 1;
        if (c == '\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        return c;
    }

    fn skipWhitespace(self: *Parser) void {
        while (self.peek()) |c| {
            if (c == ' ' or c == '\t' or c == '\r' or c == '\n') {
                _ = self.advance();
            } else {
                break;
            }
        }
    }

    fn matchPrefix(self: *Parser, prefix: []const u8) bool {
        if (self.pos + prefix.len <= self.source.len) {
            if (std.mem.eql(u8, self.source[self.pos .. self.pos + prefix.len], prefix)) {
                return true;
            }
        }
        return false;
    }

    fn consumePrefix(self: *Parser, prefix: []const u8) bool {
        if (self.matchPrefix(prefix)) {
            for (prefix) |_| _ = self.advance();
            return true;
        }
        return false;
    }

    pub fn parse(self: *Parser) ParseError!ast.Component {
        var scripts: std.ArrayList(ast.ScriptBlock) = .empty;
        var states: std.ArrayList(ast.StateVar) = .empty;
        var root_nodes: std.ArrayList(ast.Node) = .empty;
        var styles: []const u8 = "";

        while (self.pos < self.source.len) {
            self.skipWhitespace();
            if (self.pos >= self.source.len) break;

            if (self.matchPrefix("<script")) {
                const block = try self.parseScriptBlock();
                try scripts.append(self.allocator, block);
            } else if (self.matchPrefix("<state>")) {
                const parsed_states = try self.parseStateBlock();
                try states.appendSlice(self.allocator, parsed_states);
            } else if (self.matchPrefix("<style>")) {
                styles = try self.parseStyleBlock();
            } else if (self.peek() == '<') {
                const node = try self.parseElement();
                try root_nodes.append(self.allocator, node);
            } else {
                if (self.peek() == '{') {
                    const node = try self.parseInterpolation();
                    try root_nodes.append(self.allocator, node);
                } else {
                    const text = try self.parseText();
                    if (text.len > 0) {
                        try root_nodes.append(self.allocator, .{
                            .node_type = .text,
                            .content = text,
                            .line = self.line,
                        });
                    }
                }
            }
        }

        return ast.Component{
            .arena = undefined,
            .scripts = try scripts.toOwnedSlice(self.allocator),
            .states = try states.toOwnedSlice(self.allocator),
            .root_nodes = try root_nodes.toOwnedSlice(self.allocator),
            .styles = styles,
        };
    }

    fn parseScriptBlock(self: *Parser) ParseError!ast.ScriptBlock {
        const start_line = self.line;
        const start_col = self.col;
        _ = self.consumePrefix("<script");

        var lang: ?ast.ScriptLang = null;
        var context: ?ast.ScriptContext = null;

        while (self.pos < self.source.len and self.peek() != '>') {
            self.skipWhitespace();
            if (self.peek() == '>') break;

            const attr_name_start = self.pos;
            while (self.peek()) |c| {
                if (c == '=' or c == ' ' or c == '>') break;
                _ = self.advance();
            }
            const attr_name = self.source[attr_name_start..self.pos];
            self.skipWhitespace();

            if (self.peek() == '=') {
                _ = self.advance();
                self.skipWhitespace();
                if (self.peek() == '"') {
                    _ = self.advance();
                    const val_start = self.pos;
                    while (self.peek()) |c| {
                        if (c == '"') break;
                        _ = self.advance();
                    }
                    const val = self.source[val_start..self.pos];
                    if (self.peek() == '"') _ = self.advance();

                    if (std.mem.eql(u8, attr_name, "lang")) {
                        lang = ast.ScriptLang.fromString(val);
                    } else if (std.mem.eql(u8, attr_name, "context")) {
                        context = ast.ScriptContext.fromString(val);
                    }
                }
            }
        }

        if (self.peek() == '>') _ = self.advance();

        if (lang == null or context == null) {
            return ParseError.MissingScriptAttribute;
        }

        const body_start = self.pos;
        while (self.pos < self.source.len) {
            if (self.matchPrefix("</script>")) {
                const body = std.mem.trim(u8, self.source[body_start..self.pos], " \r\n");
                _ = self.consumePrefix("</script>");
                return ast.ScriptBlock{
                    .lang = lang.?,
                    .context = context.?,
                    .code = body,
                    .line = start_line,
                    .col = start_col,
                };
            }
            _ = self.advance();
        }

        return ParseError.UnterminatedScript;
    }

    fn parseStateBlock(self: *Parser) ParseError![]ast.StateVar {
        _ = self.consumePrefix("<state>");
        const body_start = self.pos;
        while (self.pos < self.source.len and !self.matchPrefix("</state>")) {
            _ = self.advance();
        }
        if (!self.matchPrefix("</state>")) return ParseError.UnterminatedState;

        const body = self.source[body_start..self.pos];
        _ = self.consumePrefix("</state>");

        var list: std.ArrayList(ast.StateVar) = .empty;
        var lines_it = std.mem.splitScalar(u8, body, '\n');
        var cur_line: usize = self.line;

        while (lines_it.next()) |raw_line| {
            cur_line += 1;
            const line = std.mem.trim(u8, raw_line, " \t\r");
            if (line.len == 0 or line[0] == '#') continue;

            const colon_idx = std.mem.indexOfScalar(u8, line, ':') orelse continue;
            const name = std.mem.trim(u8, line[0..colon_idx], " \t");

            const rest = line[colon_idx + 1 ..];
            const eq_idx = std.mem.indexOfScalar(u8, rest, '=') orelse continue;
            const type_name = std.mem.trim(u8, rest[0..eq_idx], " \t");
            const initial_value = std.mem.trim(u8, rest[eq_idx + 1 ..], " \t");

            try list.append(self.allocator, .{
                .name = name,
                .type_name = type_name,
                .initial_value = initial_value,
                .line = cur_line,
            });
        }

        return list.toOwnedSlice(self.allocator);
    }

    fn parseStyleBlock(self: *Parser) ParseError![]const u8 {
        _ = self.consumePrefix("<style>");
        const body_start = self.pos;
        while (self.pos < self.source.len and !self.matchPrefix("</style>")) {
            _ = self.advance();
        }
        if (!self.matchPrefix("</style>")) return ParseError.UnterminatedStyle;

        const body = std.mem.trim(u8, self.source[body_start..self.pos], " \r\n");
        _ = self.consumePrefix("</style>");
        return body;
    }

    fn parseElement(self: *Parser) ParseError!ast.Node {
        const start_line = self.line;
        _ = self.advance(); // consume <

        const tag_start = self.pos;
        while (self.peek()) |c| {
            if (c == ' ' or c == '\t' or c == '\n' or c == '>' or c == '/') break;
            _ = self.advance();
        }
        const tag = self.source[tag_start..self.pos];
        if (tag.len == 0) return ParseError.InvalidTag;

        var attrs: std.ArrayList(ast.Attribute) = .empty;
        var events: std.ArrayList(ast.EventBinding) = .empty;

        var self_closing = false;
        while (self.pos < self.source.len) {
            self.skipWhitespace();
            if (self.matchPrefix("/>")) {
                _ = self.consumePrefix("/>");
                self_closing = true;
                break;
            }
            if (self.peek() == '>') {
                _ = self.advance();
                break;
            }

            const attr_name_start = self.pos;
            while (self.peek()) |c| {
                if (c == '=' or c == ' ' or c == '>' or c == '/') break;
                _ = self.advance();
            }
            const attr_name = self.source[attr_name_start..self.pos];
            self.skipWhitespace();

            if (self.peek() == '=') {
                _ = self.advance();
                self.skipWhitespace();
                if (self.peek() == '{') {
                    _ = self.advance();
                    const expr_start = self.pos;
                    var brace_count: usize = 1;
                    while (self.pos < self.source.len and brace_count > 0) {
                        if (self.peek() == '{') brace_count += 1;
                        if (self.peek() == '}') brace_count -= 1;
                        if (brace_count == 0) break;
                        _ = self.advance();
                    }
                    const expr = self.source[expr_start..self.pos];
                    if (self.peek() == '}') _ = self.advance();

                    try events.append(self.allocator, .{
                        .event_name = attr_name,
                        .expression = expr,
                        .line = self.line,
                    });
                } else if (self.peek() == '"') {
                    _ = self.advance();
                    const val_start = self.pos;
                    while (self.peek()) |c| {
                        if (c == '"') break;
                        _ = self.advance();
                    }
                    const val = self.source[val_start..self.pos];
                    if (self.peek() == '"') _ = self.advance();

                    try attrs.append(self.allocator, .{
                        .name = attr_name,
                        .value = val,
                    });
                }
            } else {
                try attrs.append(self.allocator, .{
                    .name = attr_name,
                    .value = "true",
                });
            }
        }

        const is_void = std.mem.eql(u8, tag, "input") or
            std.mem.eql(u8, tag, "img") or
            std.mem.eql(u8, tag, "br") or
            std.mem.eql(u8, tag, "hr");

        if (self_closing or is_void) {
            return ast.Node{
                .node_type = .element,
                .tag = tag,
                .attributes = try attrs.toOwnedSlice(self.allocator),
                .events = try events.toOwnedSlice(self.allocator),
                .children = &.{},
                .line = start_line,
            };
        }

        var children: std.ArrayList(ast.Node) = .empty;
        const close_tag_pattern = try std.fmt.allocPrint(self.allocator, "</{s}>", .{tag});
        defer self.allocator.free(close_tag_pattern);

        while (self.pos < self.source.len) {
            self.skipWhitespace();
            if (self.matchPrefix(close_tag_pattern)) {
                _ = self.consumePrefix(close_tag_pattern);
                break;
            }

            if (self.peek() == '<') {
                const child = try self.parseElement();
                try children.append(self.allocator, child);
            } else if (self.peek() == '{') {
                const child = try self.parseInterpolation();
                try children.append(self.allocator, child);
            } else {
                const text = try self.parseText();
                if (text.len > 0) {
                    try children.append(self.allocator, .{
                        .node_type = .text,
                        .content = text,
                        .line = self.line,
                    });
                }
            }
        }

        return ast.Node{
            .node_type = .element,
            .tag = tag,
            .attributes = try attrs.toOwnedSlice(self.allocator),
            .events = try events.toOwnedSlice(self.allocator),
            .children = try children.toOwnedSlice(self.allocator),
            .line = start_line,
        };
    }

    fn parseInterpolation(self: *Parser) ParseError!ast.Node {
        const start_line = self.line;
        _ = self.advance();
        const expr_start = self.pos;
        var brace_depth: usize = 1;

        while (self.pos < self.source.len and brace_depth > 0) {
            if (self.peek() == '{') brace_depth += 1;
            if (self.peek() == '}') brace_depth -= 1;
            if (brace_depth == 0) break;
            _ = self.advance();
        }
        const expr = std.mem.trim(u8, self.source[expr_start..self.pos], " \t\r\n");
        if (self.peek() == '}') _ = self.advance();

        return ast.Node{
            .node_type = .interpolation,
            .content = expr,
            .line = start_line,
        };
    }

    fn parseText(self: *Parser) ParseError![]const u8 {
        const text_start = self.pos;
        while (self.peek()) |c| {
            if (c == '<' or c == '{') break;
            _ = self.advance();
        }
        return std.mem.trim(u8, self.source[text_start..self.pos], " \t\r\n");
    }
};

pub fn parseComponent(child_allocator: Allocator, source: []const u8) ParseError!ast.Component {
    const arena = try child_allocator.create(std.heap.ArenaAllocator);
    arena.* = std.heap.ArenaAllocator.init(child_allocator);
    errdefer {
        arena.deinit();
        child_allocator.destroy(arena);
    }
    const alloc = arena.allocator();

    var p = Parser.init(alloc, source);
    var comp = try p.parse();
    comp.arena = arena;
    return comp;
}

test "parse static hello world" {
    const allocator = std.testing.allocator;
    const src = "<h1>Hello Zvelte</h1>";
    var comp = try parseComponent(allocator, src);
    defer comp.deinit(allocator);

    try std.testing.expectEqual(@as(usize, 1), comp.root_nodes.len);
    try std.testing.expectEqualStrings("h1", comp.root_nodes[0].tag);
    try std.testing.expectEqual(@as(usize, 1), comp.root_nodes[0].children.len);
    try std.testing.expectEqualStrings("Hello Zvelte", comp.root_nodes[0].children[0].content);
}

test "parse full hybrid app.zvelte" {
    const allocator = std.testing.allocator;
    const src =
        \\<script lang="python" context="server">
        \\name = "Babbar"
        \\def greeting():
        \\    return f"Hello {name}"
        \\</script>
        \\
        \\<script lang="zig" context="native">
        \\pub export fn zvelte_double(value: i32) callconv(.c) i32 {
        \\    return value * 2;
        \\}
        \\</script>
        \\
        \\<script lang="zig" context="wasm">
        \\pub export fn zvelte_square(value: i32) callconv(.c) i32 {
        \\    return value * value;
        \\}
        \\</script>
        \\
        \\<state>
        \\count: i32 = 0
        \\</state>
        \\
        \\<main>
        \\    <h1>{greeting()}</h1>
        \\    <h2>Count: {count}</h2>
        \\    <button click={count++}>Increment</button>
        \\    <button click={count = 0}>Reset</button>
        \\</main>
        \\
        \\<style>
        \\main {
        \\    font-family: system-ui, sans-serif;
        \\}
        \\</style>
    ;

    var comp = try parseComponent(allocator, src);
    defer comp.deinit(allocator);

    try std.testing.expectEqual(@as(usize, 3), comp.scripts.len);
    try std.testing.expectEqual(ast.ScriptLang.python, comp.scripts[0].lang);
    try std.testing.expectEqual(ast.ScriptContext.server, comp.scripts[0].context);
    try std.testing.expectEqual(ast.ScriptLang.zig, comp.scripts[1].lang);
    try std.testing.expectEqual(ast.ScriptContext.native, comp.scripts[1].context);
    try std.testing.expectEqual(ast.ScriptLang.zig, comp.scripts[2].lang);
    try std.testing.expectEqual(ast.ScriptContext.wasm, comp.scripts[2].context);

    try std.testing.expectEqual(@as(usize, 1), comp.states.len);
    try std.testing.expectEqualStrings("count", comp.states[0].name);
    try std.testing.expectEqualStrings("i32", comp.states[0].type_name);
    try std.testing.expectEqualStrings("0", comp.states[0].initial_value);

    try std.testing.expectEqual(@as(usize, 1), comp.root_nodes.len);
    try std.testing.expectEqualStrings("main", comp.root_nodes[0].tag);

    try std.testing.expect(comp.styles.len > 0);
}
