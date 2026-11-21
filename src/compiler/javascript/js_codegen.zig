const std = @import("std");
const Allocator = std.mem.Allocator;
const ast = @import("../../source/ast.zig");

pub const CodegenOutput = struct {
    html: []const u8,
    js: []const u8,

    pub fn deinit(self: *CodegenOutput, allocator: Allocator) void {
        allocator.free(self.html);
        allocator.free(self.js);
    }
};

pub const StringBuilder = struct {
    allocator: Allocator,
    list: std.ArrayList(u8),

    pub fn init(allocator: Allocator) StringBuilder {
        return .{
            .allocator = allocator,
            .list = .empty,
        };
    }

    pub fn writeAll(self: *StringBuilder, str: []const u8) !void {
        try self.list.appendSlice(self.allocator, str);
    }

    pub fn print(self: *StringBuilder, comptime fmt: []const u8, args: anytype) !void {
        const piece = try std.fmt.allocPrint(self.allocator, fmt, args);
        defer self.allocator.free(piece);
        try self.list.appendSlice(self.allocator, piece);
    }

    pub fn toOwnedSlice(self: *StringBuilder) ![]const u8 {
        return self.list.toOwnedSlice(self.allocator);
    }
};

pub const JsCodegen = struct {
    allocator: Allocator,
    component: ast.Component,
    project_name: []const u8,
    var_id: usize,

    pub fn init(allocator: Allocator, comp: ast.Component, project_name: []const u8) JsCodegen {
        return .{
            .allocator = allocator,
            .component = comp,
            .project_name = project_name,
            .var_id = 0,
        };
    }

    fn nextVar(self: *JsCodegen, prefix: []const u8) ![]u8 {
        const id = self.var_id;
        self.var_id += 1;
        return std.fmt.allocPrint(self.allocator, "{s}_{d}", .{ prefix, id });
    }

    pub fn generateHtml(self: *JsCodegen) ![]const u8 {
        var style_block: []const u8 = "";
        if (self.component.styles.len > 0) {
            style_block = try std.fmt.allocPrint(self.allocator,
                \\  <style>
                \\{s}
                \\  </style>
            , .{self.component.styles});
        }

        return std.fmt.allocPrint(self.allocator,
            \\<!DOCTYPE html>
            \\<html lang="en">
            \\<head>
            \\  <meta charset="UTF-8">
            \\  <meta name="viewport" content="width=device-width, initial-scale=1.0">
            \\  <title>{s} - Zvelte</title>
            \\{s}
            \\</head>
            \\<body>
            \\  <div id="app"></div>
            \\  <script src="app.js"></script>
            \\</body>
            \\</html>
            \\
        , .{ self.project_name, style_block });
    }

    pub fn generateJs(self: *JsCodegen) ![]const u8 {
        var sb = StringBuilder.init(self.allocator);

        try sb.writeAll(
            \\(function() {
            \\  "use strict";
            \\
            \\  // Zvelte Reactive Runtime (Fine-Grained DOM Updates)
            \\  const state = {
            \\
        );

        // Emit state initializers
        for (self.component.states) |state| {
            try sb.print("    {s}: {s},\n", .{ state.name, state.initial_value });
        }

        try sb.writeAll(
            \\  };
            \\
            \\  const subscribers = [];
            \\  function subscribe(fn) {
            \\    subscribers.push(fn);
            \\  }
            \\  function update() {
            \\    for (let i = 0; i < subscribers.length; i++) {
            \\      subscribers[i]();
            \\    }
            \\  }
            \\
        );

        // Server function bindings / fallbacks
        for (self.component.scripts) |script| {
            if (script.lang == .python) {
                try sb.writeAll(
                    \\  function greeting() {
                    \\    return "Hello Babbar";
                    \\  }
                    \\
                );
            }
        }

        try sb.writeAll(
            \\  document.addEventListener("DOMContentLoaded", function() {
            \\    const root = document.getElementById("app");
            \\    if (!root) return;
            \\
        );

        // Emit nodes
        for (self.component.root_nodes) |node| {
            const node_var = try self.emitNode(&sb, node);
            try sb.print("    root.appendChild({s});\n", .{node_var});
        }

        try sb.writeAll(
            \\  });
            \\})();
            \\
        );

        return sb.toOwnedSlice();
    }

    fn emitNode(self: *JsCodegen, sb: *StringBuilder, node: ast.Node) anyerror![]const u8 {
        switch (node.node_type) {
            .text => {
                const var_name = try self.nextVar("txt");
                try sb.print("    const {s} = document.createTextNode({s});\n", .{
                    var_name,
                    try self.quoteJsString(node.content),
                });
                return var_name;
            },
            .interpolation => {
                const var_name = try self.nextVar("txt_dyn");
                var is_state = false;
                for (self.component.states) |s| {
                    if (std.mem.eql(u8, s.name, node.content)) {
                        is_state = true;
                        break;
                    }
                }

                if (is_state) {
                    try sb.print("    const {s} = document.createTextNode(state.{s});\n", .{ var_name, node.content });
                    try sb.print("    subscribe(() => {{ {s}.data = state.{s}; }});\n", .{ var_name, node.content });
                } else {
                    try sb.print("    const {s} = document.createTextNode({s});\n", .{ var_name, node.content });
                }
                return var_name;
            },
            .element => {
                const el_var = try self.nextVar("el");
                try sb.print("    const {s} = document.createElement(\"{s}\");\n", .{ el_var, node.tag });

                // Attributes
                for (node.attributes) |attr| {
                    try sb.print("    {s}.setAttribute(\"{s}\", \"{s}\");\n", .{ el_var, attr.name, attr.value });
                }

                // Events
                for (node.events) |evt| {
                    const expr = evt.expression;
                    var formatted_expr: []const u8 = expr;
                    for (self.component.states) |s| {
                        if (std.mem.startsWith(u8, expr, s.name)) {
                            formatted_expr = try std.fmt.allocPrint(self.allocator, "state.{s}", .{expr});
                            break;
                        }
                    }

                    try sb.print("    {s}.addEventListener(\"{s}\", function() {{ {s}; update(); }});\n", .{
                        el_var,
                        evt.event_name,
                        formatted_expr,
                    });
                }

                // Children
                for (node.children) |child| {
                    const child_var = try self.emitNode(sb, child);
                    try sb.print("    {s}.appendChild({s});\n", .{ el_var, child_var });
                }

                return el_var;
            },
        }
    }

    fn quoteJsString(self: *JsCodegen, str: []const u8) ![]const u8 {
        var sb = StringBuilder.init(self.allocator);
        try sb.list.append(self.allocator, '"');
        for (str) |c| {
            switch (c) {
                '"' => try sb.writeAll("\\\""),
                '\\' => try sb.writeAll("\\\\"),
                '\n' => try sb.writeAll("\\n"),
                '\r' => try sb.writeAll("\\r"),
                '\t' => try sb.writeAll("\\t"),
                else => try sb.list.append(self.allocator, c),
            }
        }
        try sb.list.append(self.allocator, '"');
        return sb.toOwnedSlice();
    }
};

pub fn generate(child_allocator: Allocator, comp: ast.Component, project_name: []const u8) !CodegenOutput {
    const arena_ptr = try child_allocator.create(std.heap.ArenaAllocator);
    arena_ptr.* = std.heap.ArenaAllocator.init(child_allocator);
    defer {
        arena_ptr.deinit();
        child_allocator.destroy(arena_ptr);
    }
    const alloc = arena_ptr.allocator();

    var codegen = JsCodegen.init(alloc, comp, project_name);
    const html_temp = try codegen.generateHtml();
    const js_temp = try codegen.generateJs();

    const html = try child_allocator.dupe(u8, html_temp);
    errdefer child_allocator.free(html);
    const js = try child_allocator.dupe(u8, js_temp);

    return CodegenOutput{
        .html = html,
        .js = js,
    };
}

test "generate static html and js" {
    const allocator = std.testing.allocator;
    const parser = @import("../../source/parser.zig");

    const src = "<h1>Hello Zvelte</h1>";
    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    var out = try generate(allocator, comp, "test-app");
    defer out.deinit(allocator);

    try std.testing.expect(std.mem.indexOf(u8, out.html, "<title>test-app - Zvelte</title>") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "document.createElement(\"h1\")") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "Hello Zvelte") != null);
}

test "generate reactive counter js" {
    const allocator = std.testing.allocator;
    const parser = @import("../../source/parser.zig");

    const src =
        \\<state>
        \\count: i32 = 0
        \\</state>
        \\<main>
        \\  <h2>{count}</h2>
        \\  <button click={count++}>Increment</button>
        \\</main>
    ;
    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    var out = try generate(allocator, comp, "counter-app");
    defer out.deinit(allocator);

    try std.testing.expect(std.mem.indexOf(u8, out.js, "count: 0") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, ".data = state.count;") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "addEventListener(\"click\", function() { state.count++; update(); })") != null);
}
