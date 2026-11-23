const std = @import("std");
const Allocator = std.mem.Allocator;
const Io = std.Io;
const Dir = std.Io.Dir;

pub const NativeParam = struct {
    name: []const u8,
    type_name: []const u8,
};

pub const NativeFunction = struct {
    name: []const u8,
    params: []const NativeParam,
    ret_type: []const u8,

    pub fn deinit(self: *const NativeFunction, allocator: Allocator) void {
        allocator.free(self.name);
        for (self.params) |p| {
            allocator.free(p.name);
            allocator.free(p.type_name);
        }
        allocator.free(self.params);
        allocator.free(self.ret_type);
    }
};

pub fn parseExportedFunctions(allocator: Allocator, code: []const u8) ![]NativeFunction {
    var functions: std.ArrayList(NativeFunction) = .empty;
    errdefer {
        for (functions.items) |f| {
            f.deinit(allocator);
        }
        functions.deinit(allocator);
    }

    var lines = std.mem.splitScalar(u8, code, '\n');
    while (lines.next()) |raw_line| {
        const line = std.mem.trim(u8, raw_line, " \t\r");
        const fn_idx = std.mem.indexOf(u8, line, "fn ") orelse continue;
        const before_fn = line[0..fn_idx];
        if (std.mem.indexOf(u8, before_fn, "export") == null) continue;

        const after_fn = line[fn_idx + 3 ..];
        const paren_idx = std.mem.indexOfScalar(u8, after_fn, '(') orelse continue;
        const fn_name = std.mem.trim(u8, after_fn[0..paren_idx], " \t");
        if (fn_name.len == 0) continue;

        const close_paren = std.mem.indexOfScalar(u8, after_fn, ')') orelse continue;
        const params_str = std.mem.trim(u8, after_fn[paren_idx + 1 .. close_paren], " \t");

        var param_list: std.ArrayList(NativeParam) = .empty;
        errdefer {
            for (param_list.items) |p| {
                allocator.free(p.name);
                allocator.free(p.type_name);
            }
            param_list.deinit(allocator);
        }

        if (params_str.len > 0) {
            var param_it = std.mem.splitScalar(u8, params_str, ',');
            while (param_it.next()) |p_raw| {
                const p_trim = std.mem.trim(u8, p_raw, " \t");
                if (std.mem.indexOfScalar(u8, p_trim, ':')) |colon| {
                    const p_name = std.mem.trim(u8, p_trim[0..colon], " \t");
                    const p_type = std.mem.trim(u8, p_trim[colon + 1 ..], " \t");
                    try param_list.append(allocator, .{
                        .name = try allocator.dupe(u8, p_name),
                        .type_name = try allocator.dupe(u8, p_type),
                    });
                }
            }
        }

        const after_close = after_fn[close_paren + 1 ..];
        var ret_type: []const u8 = "void";
        if (std.mem.indexOf(u8, after_close, "callconv(.c)")) |cc_idx| {
            const after_cc = after_close[cc_idx + 12 ..];
            const brace_idx = std.mem.indexOfScalar(u8, after_cc, '{') orelse after_cc.len;
            const r = std.mem.trim(u8, after_cc[0..brace_idx], " \t\r");
            if (r.len > 0) {
                ret_type = r;
            }
        } else {
            const brace_idx = std.mem.indexOfScalar(u8, after_close, '{') orelse after_close.len;
            const r = std.mem.trim(u8, after_close[0..brace_idx], " \t\r");
            if (r.len > 0) {
                ret_type = r;
            }
        }

        try functions.append(allocator, .{
            .name = try allocator.dupe(u8, fn_name),
            .params = try param_list.toOwnedSlice(allocator),
            .ret_type = try allocator.dupe(u8, ret_type),
        });
    }

    return functions.toOwnedSlice(allocator);
}

pub fn generateCtypesBindings(allocator: Allocator, functions: []const NativeFunction) ![]const u8 {
    var sb: std.ArrayList(u8) = .empty;
    defer sb.deinit(allocator);

    try sb.appendSlice(allocator,
        \\# === Zvelte Native Zig C-ABI ctypes Loader ===
        \\import ctypes
        \\import os
        \\import sys
        \\
        \\_native_lib = None
        \\_lib_names = ["libnative.so", "libnative.dylib", "native.dll"]
        \\for _name in _lib_names:
        \\    _candidate = os.path.join(os.path.dirname(__file__), _name)
        \\    if os.path.exists(_candidate):
        \\        try:
        \\            _native_lib = ctypes.CDLL(_candidate)
        \\            break
        \\        except Exception:
        \\            pass
        \\
        \\def _get_ctypes_type(t_name):
        \\    m = {
        \\        "i32": ctypes.c_int32,
        \\        "i64": ctypes.c_int64,
        \\        "u32": ctypes.c_uint32,
        \\        "u64": ctypes.c_uint64,
        \\        "f32": ctypes.c_float,
        \\        "f64": ctypes.c_double,
        \\        "bool": ctypes.c_bool,
        \\        "void": None,
        \\    }
        \\    return m.get(t_name, ctypes.c_int32)
        \\
        \\
    );

    for (functions) |f| {
        const header = try std.fmt.allocPrint(allocator,
            \\if _native_lib and hasattr(_native_lib, "{s}"):
            \\    _func_{s} = getattr(_native_lib, "{s}")
            \\    _func_{s}.argtypes = [
        , .{ f.name, f.name, f.name, f.name });
        defer allocator.free(header);
        try sb.appendSlice(allocator, header);

        for (f.params, 0..) |p, idx| {
            const sep = if (idx > 0) ", " else "";
            const arg_piece = try std.fmt.allocPrint(allocator, "{s}_get_ctypes_type(\"{s}\")", .{ sep, p.type_name });
            defer allocator.free(arg_piece);
            try sb.appendSlice(allocator, arg_piece);
        }

        const body = try std.fmt.allocPrint(allocator,
            \\]
            \\    _func_{s}.restype = _get_ctypes_type("{s}")
            \\    def {s}(*args):
            \\        c_args = []
            \\        for i, a in enumerate(args):
            \\            if i < len(_func_{s}.argtypes) and _func_{s}.argtypes[i] in (ctypes.c_int32, ctypes.c_int64, ctypes.c_uint32, ctypes.c_uint64):
            \\                c_args.append(int(a))
            \\            elif i < len(_func_{s}.argtypes) and _func_{s}.argtypes[i] in (ctypes.c_float, ctypes.c_double):
            \\                c_args.append(float(a))
            \\            else:
            \\                c_args.append(a)
            \\        return _func_{s}(*c_args)
            \\else:
            \\    def {s}(*args):
            \\        raise RuntimeError("Native function '{s}' not available (libnative.so not loaded)")
            \\
            \\
        , .{ f.name, f.ret_type, f.name, f.name, f.name, f.name, f.name, f.name, f.name, f.name });
        defer allocator.free(body);
        try sb.appendSlice(allocator, body);
    }

    return sb.toOwnedSlice(allocator);
}

pub fn compileNativeLibrary(allocator: Allocator, io: Io, out_dir: Dir, code: []const u8) !usize {
    _ = allocator;
    // 1. Write native.zig
    var native_file = try out_dir.createFile(io, "native.zig", .{});
    defer native_file.close(io);
    try native_file.writeStreamingAll(io, code);

    // 2. Invoke zig build-lib to compile shared library
    var child = std.process.spawn(io, .{
        .argv = &.{ "zig", "build-lib", "native.zig", "-dynamic", "-O", "ReleaseFast", "-femit-bin=libnative.so" },
        .cwd = .{ .dir = out_dir },
        .stdin = .close,
        .stdout = .pipe,
        .stderr = .pipe,
    }) catch |err| {
        std.debug.print("  \x1b[33m!\x1b[0m Warning: could not run 'zig build-lib' ({s})\n", .{@errorName(err)});
        return 0;
    };

    const term = child.wait(io) catch return 0;
    switch (term) {
        .exited => |code_status| {
            if (code_status != 0) {
                std.debug.print("  \x1b[31merror:\x1b[0m 'zig build-lib' failed with exit code {d}\n", .{code_status});
                return 0;
            }
        },
        else => return 0,
    }

    // 3. Inspect libnative.so size
    var so_file = out_dir.openFile(io, "libnative.so", .{}) catch return 0;
    defer so_file.close(io);
    const stat = so_file.stat(io) catch return 0;
    return stat.size;
}

test "parse exported zig functions" {
    const allocator = std.testing.allocator;
    const src =
        \\pub export fn zvelte_multiply(a: i32, b: i32) callconv(.c) i32 {
        \\    return a * b;
        \\}
        \\pub export fn zvelte_fast_fib(n: u32) callconv(.c) u64 {
        \\    return n;
        \\}
    ;

    const funcs = try parseExportedFunctions(allocator, src);
    defer {
        for (funcs) |f| f.deinit(allocator);
        allocator.free(funcs);
    }

    try std.testing.expectEqual(@as(usize, 2), funcs.len);
    try std.testing.expectEqualStrings("zvelte_multiply", funcs[0].name);
    try std.testing.expectEqual(@as(usize, 2), funcs[0].params.len);
    try std.testing.expectEqualStrings("a", funcs[0].params[0].name);
    try std.testing.expectEqualStrings("i32", funcs[0].params[0].type_name);
    try std.testing.expectEqualStrings("i32", funcs[0].ret_type);

    try std.testing.expectEqualStrings("zvelte_fast_fib", funcs[1].name);
    try std.testing.expectEqualStrings("u32", funcs[1].params[0].type_name);
    try std.testing.expectEqualStrings("u64", funcs[1].ret_type);
}

test "generate ctypes bindings" {
    const allocator = std.testing.allocator;
    const src = "pub export fn zvelte_add(x: i32, y: i32) callconv(.c) i32 { return x + y; }";
    const funcs = try parseExportedFunctions(allocator, src);
    defer {
        for (funcs) |f| f.deinit(allocator);
        allocator.free(funcs);
    }

    const py_code = try generateCtypesBindings(allocator, funcs);
    defer allocator.free(py_code);

    try std.testing.expect(std.mem.indexOf(u8, py_code, "ctypes.CDLL") != null);
    try std.testing.expect(std.mem.indexOf(u8, py_code, "def zvelte_add(*args):") != null);
    try std.testing.expect(std.mem.indexOf(u8, py_code, "_func_zvelte_add.argtypes = [_get_ctypes_type(\"i32\"), _get_ctypes_type(\"i32\")]") != null);
}
