const std = @import("std");
const Allocator = std.mem.Allocator;
const ast = @import("../source/ast.zig");
const config = @import("../config/config.zig");

pub const SemanticError = error{
    ZigBlockNotAllowedInPythonMode,
    PythonBlockNotAllowedInZigMode,
    DuplicateStateVariable,
    UnsupportedStateType,
    UnknownStateInEvent,
    UnknownStateInBinding,
    UnknownCollectionInEach,
    OutOfMemory,
};

pub const SemanticModel = struct {
    arena: *std.heap.ArenaAllocator,
    component: ast.Component,
    state_names: std.StringHashMap(ast.StateVar),
    func_names: std.StringHashMap(void),

    pub fn deinit(self: *SemanticModel, child_allocator: Allocator) void {
        self.arena.deinit();
        child_allocator.destroy(self.arena);
    }
};

fn isBuiltin(name: []const u8) bool {
    const builtins = [_][]const u8{
        "update",
        "console",
        "alert",
        "fetch",
        "window",
        "document",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
    };
    for (builtins) |b| {
        if (std.mem.eql(u8, name, b)) return true;
    }
    return false;
}

fn extractScriptFunctions(allocator: Allocator, script: ast.ScriptBlock, func_map: *std.StringHashMap(void)) !void {
    _ = allocator;
    var lines_it = std.mem.splitScalar(u8, script.code, '\n');
    while (lines_it.next()) |raw_line| {
        const line = std.mem.trim(u8, raw_line, " \t\r");
        if (script.lang == .python) {
            var rest: ?[]const u8 = null;
            if (std.mem.startsWith(u8, line, "def ")) {
                rest = line[4..];
            } else if (std.mem.startsWith(u8, line, "async def ")) {
                rest = line[10..];
            }
            if (rest) |r| {
                if (std.mem.indexOfScalar(u8, r, '(')) |paren_idx| {
                    const fn_name = std.mem.trim(u8, r[0..paren_idx], " \t");
                    if (fn_name.len > 0) {
                        try func_map.put(fn_name, {});
                    }
                }
            }
        } else if (script.lang == .zig) {
            if (std.mem.indexOf(u8, line, "fn ")) |fn_idx| {
                const after_fn = line[fn_idx + 3 ..];
                if (std.mem.indexOfScalar(u8, after_fn, '(')) |paren_idx| {
                    const fn_name = std.mem.trim(u8, after_fn[0..paren_idx], " \t");
                    if (fn_name.len > 0) {
                        try func_map.put(fn_name, {});
                    }
                }
            }
        }
    }
}

pub fn analyze(child_allocator: Allocator, comp: ast.Component, cfg: config.AppConfig) SemanticError!SemanticModel {
    const arena = try child_allocator.create(std.heap.ArenaAllocator);
    arena.* = std.heap.ArenaAllocator.init(child_allocator);
    errdefer {
        arena.deinit();
        child_allocator.destroy(arena);
    }
    const alloc = arena.allocator();

    // 1. Validate script blocks against configured mode
    for (comp.scripts) |script| {
        switch (cfg.app.mode) {
            .python => {
                if (script.lang != .python) {
                    return SemanticError.ZigBlockNotAllowedInPythonMode;
                }
            },
            .zig => {
                if (script.lang != .zig) {
                    return SemanticError.PythonBlockNotAllowedInZigMode;
                }
            },
            .hybrid => {},
        }
    }

    // 2. Build state symbol table & check duplicates / valid types
    var state_map = std.StringHashMap(ast.StateVar).init(alloc);
    for (comp.states) |state| {
        if (state_map.contains(state.name)) {
            return SemanticError.DuplicateStateVariable;
        }

        const valid_type = std.mem.eql(u8, state.type_name, "i32") or
            std.mem.eql(u8, state.type_name, "i64") or
            std.mem.eql(u8, state.type_name, "u32") or
            std.mem.eql(u8, state.type_name, "u64") or
            std.mem.eql(u8, state.type_name, "f32") or
            std.mem.eql(u8, state.type_name, "f64") or
            std.mem.eql(u8, state.type_name, "bool") or
            std.mem.eql(u8, state.type_name, "string") or
            std.mem.startsWith(u8, state.type_name, "[]") or
            std.mem.eql(u8, state.type_name, "list") or
            std.mem.eql(u8, state.type_name, "array");

        if (!valid_type) {
            return SemanticError.UnsupportedStateType;
        }

        try state_map.put(state.name, state);
    }

    // 3. Extract declared functions from scripts
    var func_map = std.StringHashMap(void).init(alloc);
    for (comp.scripts) |script| {
        try extractScriptFunctions(alloc, script, &func_map);
    }

    // 4. Check event expressions, bindings, and template nodes
    for (comp.root_nodes) |node| {
        try validateNode(node, &state_map, &func_map, null);
    }

    return SemanticModel{
        .arena = arena,
        .component = comp,
        .state_names = state_map,
        .func_names = func_map,
    };
}

fn validateNode(node: ast.Node, state_map: *const std.StringHashMap(ast.StateVar), func_map: *const std.StringHashMap(void), local_scope: ?[]const u8) SemanticError!void {
    // Check events
    for (node.events) |evt| {
        // e.g. "count++" or "count = 0" or "ask_ai(user_prompt)"
        var expr = std.mem.trim(u8, evt.expression, " \t\r\n");
        // extract identifier prefix
        var id_len: usize = 0;
        while (id_len < expr.len and ((expr[id_len] >= 'a' and expr[id_len] <= 'z') or
            (expr[id_len] >= 'A' and expr[id_len] <= 'Z') or
            (expr[id_len] >= '0' and expr[id_len] <= '9') or
            expr[id_len] == '_'))
        {
            id_len += 1;
        }
        if (id_len > 0) {
            const var_name = expr[0..id_len];
            const is_local = if (local_scope) |ls| std.mem.eql(u8, var_name, ls) else false;
            const is_known_func = func_map.contains(var_name) or isBuiltin(var_name);
            if (!state_map.contains(var_name) and !is_known_func and !is_local) {
                return SemanticError.UnknownStateInEvent;
            }
        }
    }

    // Check bindings
    for (node.bindings) |bind| {
        if (!state_map.contains(bind.variable)) {
            return SemanticError.UnknownStateInBinding;
        }
    }

    // Check if_block
    if (node.node_type == .if_block) {
        for (node.children) |child| {
            try validateNode(child, state_map, func_map, local_scope);
        }
        for (node.else_children) |child| {
            try validateNode(child, state_map, func_map, local_scope);
        }
        return;
    }

    // Check each_block
    if (node.node_type == .each_block) {
        if (!state_map.contains(node.content)) {
            return SemanticError.UnknownCollectionInEach;
        }
        for (node.children) |child| {
            try validateNode(child, state_map, func_map, node.item_name);
        }
        return;
    }

    for (node.children) |child| {
        try validateNode(child, state_map, func_map, local_scope);
    }
}

test "semantic analysis passes on valid hybrid counter" {
    const allocator = std.testing.allocator;
    const parser = @import("../source/parser.zig");

    const src =
        \\<script lang="python" context="server">
        \\name = "Babbar"
        \\</script>
        \\<script lang="zig" context="native">
        \\pub export fn zvelte_double(value: i32) callconv(.c) i32 { return value * 2; }
        \\</script>
        \\<state>
        \\count: i32 = 0
        \\</state>
        \\<main>
        \\  <button click={count++}>Increment</button>
        \\</main>
    ;

    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    const toml_text =
        \\[project]
        \\name = "my-app"
        \\[app]
        \\mode = "hybrid"
    ;
    var cfg = try config.loadFromSource(allocator, toml_text);
    defer cfg.deinit();

    var model = try analyze(allocator, comp, cfg);
    defer model.deinit(allocator);

    try std.testing.expect(model.state_names.contains("count"));
}

test "semantic analysis rejects zig block in python-only mode" {
    const allocator = std.testing.allocator;
    const parser = @import("../source/parser.zig");

    const src =
        \\<script lang="zig" context="native">
        \\pub export fn zvelte_double(value: i32) callconv(.c) i32 { return value * 2; }
        \\</script>
        \\<h1>Static</h1>
    ;

    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    const toml_text =
        \\[project]
        \\name = "py-only-app"
        \\[app]
        \\mode = "python"
    ;
    var cfg = try config.loadFromSource(allocator, toml_text);
    defer cfg.deinit();

    const result = analyze(allocator, comp, cfg);
    try std.testing.expectError(SemanticError.ZigBlockNotAllowedInPythonMode, result);
}

test "semantic analysis validates if, each, and bindings" {
    const allocator = std.testing.allocator;
    const parser = @import("../source/parser.zig");

    const src =
        \\<state>
        \\todos: []string = ["Task 1", "Task 2"]
        \\new_task: string = ""
        \\is_done: bool = false
        \\</state>
        \\<main>
        \\  <input type="text" bind:value={new_task} />
        \\  <input type="checkbox" bind:checked={is_done} />
        \\  {#if is_done}
        \\    <p>Completed!</p>
        \\  {:else}
        \\    <p>Pending...</p>
        \\  {/if}
        \\  {#each todos as todo}
        \\    <div>{todo}</div>
        \\  {/each}
        \\</main>
    ;

    var comp = try parser.parseComponent(allocator, src);
    defer comp.deinit(allocator);

    const toml_text =
        \\[project]
        \\name = "test-app"
        \\[app]
        \\mode = "hybrid"
    ;
    var cfg = try config.loadFromSource(allocator, toml_text);
    defer cfg.deinit();

    var model = try analyze(allocator, comp, cfg);
    defer model.deinit(allocator);

    try std.testing.expect(model.state_names.contains("todos"));
    try std.testing.expect(model.state_names.contains("new_task"));
    try std.testing.expect(model.state_names.contains("is_done"));
}

test "semantic analysis rejects unknown state in bind and each" {
    const allocator = std.testing.allocator;
    const parser = @import("../source/parser.zig");

    const src_bad_bind =
        \\<main>
        \\  <input bind:value={unknown_var} />
        \\</main>
    ;

    var comp1 = try parser.parseComponent(allocator, src_bad_bind);
    defer comp1.deinit(allocator);

    const toml_text =
        \\[project]
        \\name = "test-app"
        \\[app]
        \\mode = "hybrid"
    ;
    var cfg = try config.loadFromSource(allocator, toml_text);
    defer cfg.deinit();

    const res1 = analyze(allocator, comp1, cfg);
    try std.testing.expectError(SemanticError.UnknownStateInBinding, res1);

    const src_bad_each =
        \\<main>
        \\  {#each non_existent_list as item}
        \\    <p>{item}</p>
        \\  {/each}
        \\</main>
    ;

    var comp2 = try parser.parseComponent(allocator, src_bad_each);
    defer comp2.deinit(allocator);

    const res2 = analyze(allocator, comp2, cfg);
    try std.testing.expectError(SemanticError.UnknownCollectionInEach, res2);
}

test "semantic analysis permits declared python server functions in event expressions" {
    const allocator = std.testing.allocator;
    const parser = @import("../source/parser.zig");

    const src_valid_rpc =
        \\<script lang="python" context="server">
        \\def ask_ai(prompt):
        \\    return "Analysis: " + prompt
        \\</script>
        \\<state>
        \\query: string = "hello"
        \\ans: string = ""
        \\</state>
        \\<main>
        \\  <button click={ask_ai(query)}>Run AI</button>
        \\</main>
    ;

    var comp = try parser.parseComponent(allocator, src_valid_rpc);
    defer comp.deinit(allocator);

    const toml_text =
        \\[project]
        \\name = "ai-test"
        \\[app]
        \\mode = "hybrid"
    ;
    var cfg = try config.loadFromSource(allocator, toml_text);
    defer cfg.deinit();

    var model = try analyze(allocator, comp, cfg);
    defer model.deinit(allocator);

    try std.testing.expect(model.func_names.contains("ask_ai"));

    const src_bad_event =
        \\<state>
        \\query: string = "hello"
        \\</state>
        \\<main>
        \\  <button click={undefined_action(query)}>Run</button>
        \\</main>
    ;

    var comp_bad = try parser.parseComponent(allocator, src_bad_event);
    defer comp_bad.deinit(allocator);

    const bad_res = analyze(allocator, comp_bad, cfg);
    try std.testing.expectError(SemanticError.UnknownStateInEvent, bad_res);
}
