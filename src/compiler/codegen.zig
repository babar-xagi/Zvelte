const std = @import("std");
const Allocator = std.mem.Allocator;
const Io = std.Io;
const Dir = std.Io.Dir;

const config = @import("../config/config.zig");
const parser = @import("../source/parser.zig");
const analyzer = @import("../semantic/analyzer.zig");
const js_codegen = @import("javascript/js_codegen.zig");
const native_compiler = @import("native/native_compiler.zig");

pub const BuildResult = struct {
    output_dir: []const u8,
    html_bytes: usize,
    js_bytes: usize,
    worker_bytes: usize = 0,
    native_bytes: usize = 0,
};

pub fn generatePythonWorker(allocator: Allocator, user_code: []const u8, ctypes_code: []const u8) ![]const u8 {
    return std.fmt.allocPrint(allocator,
        \\#!/usr/bin/env python3
        \\import sys
        \\import json
        \\import traceback
        \\
        \\{s}
        \\
        \\# === Zvelte User Python Server Logic ===
        \\{s}
        \\# =======================================
        \\
        \\def handle_rpc(payload):
        \\    method = payload.get("method")
        \\    params = payload.get("params", [])
        \\    if not method:
        \\        return {{"error": "Missing 'method' in RPC request"}}
        \\    func = globals().get(method)
        \\    if not func or not callable(func):
        \\        return {{"error": f"Function '{{method}}' not found on Python server"}}
        \\    try:
        \\        if isinstance(params, list):
        \\            res = func(*params)
        \\        elif isinstance(params, dict):
        \\            res = func(**params)
        \\        else:
        \\            res = func(params)
        \\        return {{"result": res}}
        \\    except Exception as e:
        \\        return {{"error": str(e), "traceback": traceback.format_exc()}}
        \\
        \\def main():
        \\    while True:
        \\        line = sys.stdin.readline()
        \\        if not line:
        \\            break
        \\        line = line.strip()
        \\        if not line:
        \\            continue
        \\        try:
        \\            req = json.loads(line)
        \\            res = handle_rpc(req)
        \\            if "id" in req:
        \\                res["id"] = req["id"]
        \\            sys.stdout.write(json.dumps(res) + "\n")
        \\            sys.stdout.flush()
        \\        except Exception as err:
        \\            sys.stdout.write(json.dumps({{"error": str(err)}}) + "\n")
        \\            sys.stdout.flush()
        \\
        \\if __name__ == "__main__":
        \\    main()
        \\
    , .{ ctypes_code, user_code });
}

pub fn buildProject(
    allocator: Allocator,
    io: Io,
    project_dir: Dir,
    output_dir_name: []const u8,
) !BuildResult {
    // 1. Read app.toml
    const toml_source = try project_dir.readFileAlloc(io, "app.toml", allocator, .unlimited);
    defer allocator.free(toml_source);

    var cfg = try config.loadFromSource(allocator, toml_source);
    defer cfg.deinit();

    // 2. Read entry file (app.zvelte)
    const zvelte_source = try project_dir.readFileAlloc(io, cfg.project.entry, allocator, .unlimited);
    defer allocator.free(zvelte_source);

    // 3. Parse component
    var comp = try parser.parseComponent(allocator, zvelte_source);
    defer comp.deinit(allocator);

    // 4. Semantic analysis
    var sem_model = try analyzer.analyze(allocator, comp, cfg);
    defer sem_model.deinit(allocator);

    // 5. Code generation
    var out = try js_codegen.generate(allocator, comp, cfg.project.name);
    defer out.deinit(allocator);

    // 6. Output directory
    try project_dir.createDirPath(io, output_dir_name);
    var out_dir = try project_dir.openDir(io, output_dir_name, .{});
    defer out_dir.close(io);

    // Write index.html
    var html_file = try out_dir.createFile(io, "index.html", .{});
    defer html_file.close(io);
    try html_file.writeStreamingAll(io, out.html);

    // Write app.js
    var js_file = try out_dir.createFile(io, "app.js", .{});
    defer js_file.close(io);
    try js_file.writeStreamingAll(io, out.js);

    // Check for native Zig script blocks & compile libnative.so
    var native_bytes: usize = 0;
    var ctypes_code: []const u8 = "";
    var has_native = false;
    for (comp.scripts) |script| {
        if (script.lang == .zig and script.context == .native) {
            has_native = true;
            native_bytes = native_compiler.compileNativeLibrary(allocator, io, out_dir, script.code) catch 0;
            const native_funcs = try native_compiler.parseExportedFunctions(allocator, script.code);
            defer {
                for (native_funcs) |f| f.deinit(allocator);
                allocator.free(native_funcs);
            }
            ctypes_code = try native_compiler.generateCtypesBindings(allocator, native_funcs);
            break;
        }
    }
    defer if (has_native) allocator.free(ctypes_code);

    var worker_bytes: usize = 0;
    // Write worker.py if python server scripts exist
    for (comp.scripts) |script| {
        if (script.lang == .python and script.context == .server) {
            const worker_code = try generatePythonWorker(allocator, script.code, ctypes_code);
            defer allocator.free(worker_code);

            var worker_file = try out_dir.createFile(io, "worker.py", .{});
            defer worker_file.close(io);
            try worker_file.writeStreamingAll(io, worker_code);
            worker_bytes = worker_code.len;
            break;
        }
    }

    // If native code exists without python server script, still emit worker.py
    if (worker_bytes == 0 and has_native) {
        const worker_code = try generatePythonWorker(allocator, "", ctypes_code);
        defer allocator.free(worker_code);

        var worker_file = try out_dir.createFile(io, "worker.py", .{});
        defer worker_file.close(io);
        try worker_file.writeStreamingAll(io, worker_code);
        worker_bytes = worker_code.len;
    }

    return BuildResult{
        .output_dir = output_dir_name,
        .html_bytes = out.html.len,
        .js_bytes = out.js.len,
        .worker_bytes = worker_bytes,
        .native_bytes = native_bytes,
    };
}

test "generate python worker script" {
    const allocator = std.testing.allocator;
    const user_code = "def greeting(): return 'Hello from test'";
    const ctypes_code = "# test ctypes";
    const worker = try generatePythonWorker(allocator, user_code, ctypes_code);
    defer allocator.free(worker);

    try std.testing.expect(std.mem.indexOf(u8, worker, "def handle_rpc(payload):") != null);
    try std.testing.expect(std.mem.indexOf(u8, worker, "def greeting(): return 'Hello from test'") != null);
    try std.testing.expect(std.mem.indexOf(u8, worker, "# test ctypes") != null);
    try std.testing.expect(std.mem.indexOf(u8, worker, "sys.stdin.readline()") != null);
}

test "python worker rpc execution" {
    const allocator = std.testing.allocator;
    const io = std.testing.io;
    const user_code =
        \\def add(a, b):
        \\    return int(a) + int(b)
        \\def greet(name):
        \\    return f"Welcome {name}!"
    ;
    const worker_code = try generatePythonWorker(allocator, user_code, "");
    defer allocator.free(worker_code);

    var child = try std.process.spawn(io, .{
        .argv = &.{ "python3", "-u", "-c", worker_code },
        .stdin = .pipe,
        .stdout = .pipe,
    });
    defer child.kill(io);

    // Call add(15, 25)
    try child.stdin.?.writeStreamingAll(io, "{\"method\": \"add\", \"params\": [15, 25]}\n");
    var buf1: [256]u8 = undefined;
    const n1 = try child.stdout.?.readStreaming(io, &.{&buf1});
    try std.testing.expect(std.mem.indexOf(u8, buf1[0..n1], "{\"result\": 40}") != null);

    // Call greet("Alice")
    try child.stdin.?.writeStreamingAll(io, "{\"method\": \"greet\", \"params\": [\"Alice\"]}\n");
    var buf2: [256]u8 = undefined;
    const n2 = try child.stdout.?.readStreaming(io, &.{&buf2});
    try std.testing.expect(std.mem.indexOf(u8, buf2[0..n2], "{\"result\": \"Welcome Alice!\"}") != null);

    // Call unknown method
    try child.stdin.?.writeStreamingAll(io, "{\"method\": \"non_existent\", \"params\": []}\n");
    var buf3: [256]u8 = undefined;
    const n3 = try child.stdout.?.readStreaming(io, &.{&buf3});
    try std.testing.expect(std.mem.indexOf(u8, buf3[0..n3], "Function 'non_existent' not found") != null);
}
