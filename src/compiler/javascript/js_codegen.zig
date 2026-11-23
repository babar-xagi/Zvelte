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
    in_dynamic_block: bool,

    pub fn init(allocator: Allocator, comp: ast.Component, project_name: []const u8) JsCodegen {
        return .{
            .allocator = allocator,
            .component = comp,
            .project_name = project_name,
            .var_id = 0,
            .in_dynamic_block = false,
        };
    }

    fn nextVar(self: *JsCodegen, prefix: []const u8) ![]u8 {
        const id = self.var_id;
        self.var_id += 1;
        return std.fmt.allocPrint(self.allocator, "{s}_{d}", .{ prefix, id });
    }

    pub fn formatExpr(self: *JsCodegen, expr: []const u8) ![]const u8 {
        var sb = StringBuilder.init(self.allocator);
        var i: usize = 0;
        while (i < expr.len) {
            const c = expr[i];
            if ((c >= 'a' and c <= 'z') or (c >= 'A' and c <= 'Z') or c == '_') {
                const start = i;
                while (i < expr.len and ((expr[i] >= 'a' and expr[i] <= 'z') or
                    (expr[i] >= 'A' and expr[i] <= 'Z') or
                    (expr[i] >= '0' and expr[i] <= '9') or
                    expr[i] == '_'))
                {
                    i += 1;
                }
                const ident = expr[start..i];
                const is_property_access = if (start > 0 and expr[start - 1] == '.') true else false;

                var is_state = false;
                if (!is_property_access) {
                    for (self.component.states) |s| {
                        if (std.mem.eql(u8, s.name, ident)) {
                            is_state = true;
                            break;
                        }
                    }
                }

                if (is_state) {
                    try sb.writeAll("state.");
                    try sb.writeAll(ident);
                } else {
                    try sb.writeAll(ident);
                }
            } else {
                try sb.list.append(self.allocator, c);
                i += 1;
            }
        }
        return sb.toOwnedSlice();
    }

    pub fn extractPythonFunctionNames(allocator: Allocator, code: []const u8) ![][]const u8 {
        var names: std.ArrayList([]const u8) = .empty;
        var lines_it = std.mem.splitScalar(u8, code, '\n');
        while (lines_it.next()) |raw_line| {
            const line = std.mem.trim(u8, raw_line, " \t\r");
            if (std.mem.startsWith(u8, line, "def ")) {
                const rest = line[4..];
                if (std.mem.indexOfScalar(u8, rest, '(')) |paren_idx| {
                    const fn_name = std.mem.trim(u8, rest[0..paren_idx], " \t");
                    if (fn_name.len > 0) {
                        try names.append(allocator, fn_name);
                    }
                }
            }
        }
        return names.toOwnedSlice(allocator);
    }

    pub fn extractZigFunctionNames(allocator: Allocator, code: []const u8) ![][]const u8 {
        var names: std.ArrayList([]const u8) = .empty;
        var lines_it = std.mem.splitScalar(u8, code, '\n');
        while (lines_it.next()) |raw_line| {
            const line = std.mem.trim(u8, raw_line, " \t\r");
            const fn_idx = std.mem.indexOf(u8, line, "fn ") orelse continue;
            const before_fn = line[0..fn_idx];
            if (std.mem.indexOf(u8, before_fn, "export") == null) continue;
            const after_fn = line[fn_idx + 3 ..];
            if (std.mem.indexOfScalar(u8, after_fn, '(')) |paren_idx| {
                const fn_name = std.mem.trim(u8, after_fn[0..paren_idx], " \t");
                if (fn_name.len > 0) {
                    try names.append(allocator, fn_name);
                }
            }
        }
        return names.toOwnedSlice(allocator);
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
        var has_server_rpc = false;
        for (self.component.scripts) |script| {
            if ((script.lang == .python and script.context == .server) or
                (script.lang == .zig and script.context == .native))
            {
                has_server_rpc = true;
                break;
            }
        }

        if (has_server_rpc) {
            try sb.writeAll(
                \\  // Zvelte Server & Native RPC Bridge
                \\  async function __zvelte_rpc(method, params) {
                \\    try {
                \\      const response = await fetch("/api/rpc", {
                \\        method: "POST",
                \\        headers: { "Content-Type": "application/json" },
                \\        body: JSON.stringify({ method: method, params: params || [] })
                \\      });
                \\      const data = await response.json();
                \\      if (data && data.error) {
                \\        console.error("[Zvelte RPC Error]", data.error);
                \\        return "RPC Error: " + data.error;
                \\      }
                \\      return data ? data.result : "No response data";
                \\    } catch (err) {
                \\      console.error("[Zvelte RPC Network Error]", err);
                \\      return "Network Error: " + err;
                \\    }
                \\  }
                \\  window.__zvelte_rpc = __zvelte_rpc;
                \\
            );

            for (self.component.scripts) |script| {
                if (script.lang == .python and script.context == .server) {
                    const fn_names = try extractPythonFunctionNames(self.allocator, script.code);
                    for (fn_names) |fn_name| {
                        try sb.print(
                            \\  async function {s}(...args) {{
                            \\    return await __zvelte_rpc("{s}", args);
                            \\  }}
                            \\  window.{s} = {s};
                            \\
                        , .{ fn_name, fn_name, fn_name, fn_name });
                    }
                } else if (script.lang == .zig and script.context == .native) {
                    const fn_names = try extractZigFunctionNames(self.allocator, script.code);
                    for (fn_names) |fn_name| {
                        try sb.print(
                            \\  async function {s}(...args) {{
                            \\    return await __zvelte_rpc("{s}", args);
                            \\  }}
                            \\  window.{s} = {s};
                            \\
                        , .{ fn_name, fn_name, fn_name, fn_name });
                    }
                }
            }
        }

        try sb.writeAll(
            \\  document.addEventListener("DOMContentLoaded", function() {
            \\    const root = document.getElementById("app");
            \\    if (!root) return;
            \\
            \\    function __setNodeText(node, val) {
            \\      if (val && typeof val.then === "function") {
            \\        val.then(res => { node.data = res != null ? res : ""; });
            \\      } else {
            \\        node.data = val != null ? val : "";
            \\      }
            \\    }
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
                const formatted = try self.formatExpr(node.content);

                try sb.print("    const {s} = document.createTextNode(\"\");\n", .{var_name});
                try sb.print("    __setNodeText({s}, {s});\n", .{ var_name, formatted });
                if (!self.in_dynamic_block) {
                    try sb.print("    subscribe(() => {{ __setNodeText({s}, {s}); }});\n", .{ var_name, formatted });
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

                // Two-way bindings
                for (node.bindings) |bind| {
                    if (std.mem.eql(u8, bind.property, "checked")) {
                        try sb.print("    {s}.checked = state.{s};\n", .{ el_var, bind.variable });
                        try sb.print("    subscribe(() => {{ if ({s}.checked !== state.{s}) {s}.checked = state.{s}; }});\n", .{
                            el_var, bind.variable, el_var, bind.variable,
                        });
                        try sb.print("    {s}.addEventListener(\"change\", function(e) {{ state.{s} = e.target.checked; update(); }});\n", .{
                            el_var, bind.variable,
                        });
                    } else {
                        try sb.print("    {s}.value = state.{s};\n", .{ el_var, bind.variable });
                        try sb.print("    subscribe(() => {{ if ({s}.value !== state.{s}) {s}.value = state.{s}; }});\n", .{
                            el_var, bind.variable, el_var, bind.variable,
                        });
                        try sb.print("    {s}.addEventListener(\"input\", function(e) {{ state.{s} = e.target.value; update(); }});\n", .{
                            el_var, bind.variable,
                        });
                    }
                }

                // Events
                for (node.events) |evt| {
                    const formatted_expr = try self.formatExpr(evt.expression);
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
            .if_block => {
                const if_container = try self.nextVar("if_cont");
                const fn_name = try self.nextVar("update_if");
                const formatted_cond = try self.formatExpr(node.content);

                try sb.print("    const {s} = document.createElement(\"div\");\n", .{if_container});
                try sb.print("    {s}.style.display = \"contents\";\n", .{if_container});

                try sb.print("    function {s}() {{\n", .{fn_name});
                try sb.print("      while ({s}.firstChild) {s}.removeChild({s}.firstChild);\n", .{ if_container, if_container, if_container });
                try sb.print("      if ({s}) {{\n", .{formatted_cond});

                const prev_dynamic = self.in_dynamic_block;
                self.in_dynamic_block = true;

                for (node.children) |child| {
                    const child_var = try self.emitNode(sb, child);
                    try sb.print("        {s}.appendChild({s});\n", .{ if_container, child_var });
                }

                if (node.else_children.len > 0) {
                    try sb.writeAll("      } else {\n");
                    for (node.else_children) |child| {
                        const child_var = try self.emitNode(sb, child);
                        try sb.print("        {s}.appendChild({s});\n", .{ if_container, child_var });
                    }
                }

                self.in_dynamic_block = prev_dynamic;

                try sb.writeAll("      }\n");
                try sb.writeAll("    }\n");
                try sb.print("    {s}();\n", .{fn_name});
                try sb.print("    subscribe({s});\n", .{fn_name});

                return if_container;
            },
            .each_block => {
                const each_container = try self.nextVar("each_cont");
                const fn_name = try self.nextVar("update_each");
                const loop_idx = try self.nextVar("i");
                const list_var = try self.nextVar("list");
                const formatted_coll = try self.formatExpr(node.content);

                try sb.print("    const {s} = document.createElement(\"div\");\n", .{each_container});
                try sb.print("    {s}.style.display = \"contents\";\n", .{each_container});

                try sb.print("    function {s}() {{\n", .{fn_name});
                try sb.print("      while ({s}.firstChild) {s}.removeChild({s}.firstChild);\n", .{ each_container, each_container, each_container });
                try sb.print("      const {s} = {s} || [];\n", .{ list_var, formatted_coll });
                try sb.print("      for (let {s} = 0; {s} < {s}.length; {s}++) {{\n", .{ loop_idx, loop_idx, list_var, loop_idx });
                try sb.print("        const {s} = {s}[{s}];\n", .{ node.item_name, list_var, loop_idx });

                const prev_dynamic = self.in_dynamic_block;
                self.in_dynamic_block = true;

                for (node.children) |child| {
                    const child_var = try self.emitNode(sb, child);
                    try sb.print("        {s}.appendChild({s});\n", .{ each_container, child_var });
                }

                self.in_dynamic_block = prev_dynamic;

                try sb.writeAll("      }\n");
                try sb.writeAll("    }\n");
                try sb.print("    {s}();\n", .{fn_name});
                try sb.print("    subscribe({s});\n", .{fn_name});

                return each_container;
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
    try std.testing.expect(std.mem.indexOf(u8, out.js, "__setNodeText(") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "state.count") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "addEventListener(\"click\", function() { state.count++; update(); })") != null);
}

test "generate js with if-else conditional block" {
    const allocator = std.testing.allocator;
    const parser = @import("../../source/parser.zig");

    const src =
        \\<state>
        \\count: i32 = 10
        \\</state>
        \\<main>
        \\  {#if count > 5}
        \\    <p>High</p>
        \\  {:else}
        \\    <p>Low</p>
        \\  {/if}
        \\</main>
    ;
    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    var out = try generate(allocator, comp, "if-app");
    defer out.deinit(allocator);

    try std.testing.expect(std.mem.indexOf(u8, out.js, "display = \"contents\"") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "if (state.count > 5)") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "High") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "Low") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "subscribe(update_if_") != null);
}

test "generate js with each block loop" {
    const allocator = std.testing.allocator;
    const parser = @import("../../source/parser.zig");

    const src =
        \\<state>
        \\items: []string = ["Alpha", "Beta"]
        \\</state>
        \\<main>
        \\  {#each items as item}
        \\    <div>{item}</div>
        \\  {/each}
        \\</main>
    ;
    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    var out = try generate(allocator, comp, "each-app");
    defer out.deinit(allocator);

    try std.testing.expect(std.mem.indexOf(u8, out.js, "const list_") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "for (let i_") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "const item = list_") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "subscribe(update_each_") != null);
}

test "generate js with two-way form input bindings" {
    const allocator = std.testing.allocator;
    const parser = @import("../../source/parser.zig");

    const src =
        \\<state>
        \\username: string = "Alice"
        \\agreed: bool = false
        \\</state>
        \\<main>
        \\  <input type="text" bind:value={username} />
        \\  <input type="checkbox" bind:checked={agreed} />
        \\</main>
    ;
    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    var out = try generate(allocator, comp, "bind-app");
    defer out.deinit(allocator);

    try std.testing.expect(std.mem.indexOf(u8, out.js, ".value = state.username;") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "state.username = e.target.value; update();") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, ".checked = state.agreed;") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "state.agreed = e.target.checked; update();") != null);
}

test "generate python rpc bridge and proxies" {
    const allocator = std.testing.allocator;
    const parser = @import("../../source/parser.zig");

    const src =
        \\<script lang="python" context="server">
        \\def ask_ai(prompt):
        \\    return f"AI: {prompt}"
        \\def greeting():
        \\    return "Hello from server"
        \\</script>
        \\<main>
        \\  <h1>{greeting()}</h1>
        \\</main>
    ;
    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    var out = try generate(allocator, comp, "py-app");
    defer out.deinit(allocator);

    try std.testing.expect(std.mem.indexOf(u8, out.js, "__zvelte_rpc(method, params)") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "fetch(\"/api/rpc\"") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "async function ask_ai(...args)") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "async function greeting(...args)") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.js, "__setNodeText(") != null);
}
