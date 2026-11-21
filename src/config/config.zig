const std = @import("std");
const Allocator = std.mem.Allocator;
const toml = @import("toml.zig");

pub const Mode = enum {
    python,
    zig,
    hybrid,

    pub fn fromString(str: []const u8) ?Mode {
        if (std.mem.eql(u8, str, "python")) return .python;
        if (std.mem.eql(u8, str, "zig")) return .zig;
        if (std.mem.eql(u8, str, "hybrid")) return .hybrid;
        return null;
    }
};

pub const Renderer = enum {
    js,
    wasm,
    auto,

    pub fn fromString(str: []const u8) ?Renderer {
        if (std.mem.eql(u8, str, "js")) return .js;
        if (std.mem.eql(u8, str, "wasm")) return .wasm;
        if (std.mem.eql(u8, str, "auto")) return .auto;
        return null;
    }
};

pub const Optimization = enum {
    debug,
    release_safe,
    release_fast,

    pub fn fromString(str: []const u8) ?Optimization {
        if (std.mem.eql(u8, str, "debug")) return .debug;
        if (std.mem.eql(u8, str, "release-safe")) return .release_safe;
        if (std.mem.eql(u8, str, "release-fast")) return .release_fast;
        return null;
    }
};

pub const ProjectConfig = struct {
    name: []const u8,
    version: []const u8,
    entry: []const u8,
};

pub const AppSettings = struct {
    target: []const u8,
    mode: Mode,
    renderer: Renderer,
    strict: bool,
};

pub const ToolchainConfig = struct {
    zig: []const u8,
    python: []const u8,
};

pub const PythonConfig = struct {
    enabled: bool,
    executable: []const u8,
    execution: []const u8,
    zig_acceleration: bool,
};

pub const ZigConfig = struct {
    enabled: bool,
    executable: []const u8,
    native_modules: bool,
    wasm_modules: bool,
};

pub const DevConfig = struct {
    host: []const u8,
    port: u16,
    hot_reload: bool,
    open_browser: bool,
};

pub const BuildConfig = struct {
    output: []const u8,
    cache: []const u8,
    optimization: Optimization,
    minify: bool,
    source_maps: bool,
};

pub const ConfigError = error{
    MissingProjectSection,
    MissingProjectName,
    InvalidMode,
    InvalidRenderer,
    InvalidPort,
    OutOfMemory,
} || toml.TomlError;

pub const AppConfig = struct {
    child_allocator: Allocator,
    arena: *std.heap.ArenaAllocator,
    project: ProjectConfig,
    app: AppSettings,
    toolchain: ToolchainConfig,
    python: PythonConfig,
    zig: ZigConfig,
    dev: DevConfig,
    build: BuildConfig,

    pub fn deinit(self: *AppConfig) void {
        const child_alloc = self.child_allocator;
        self.arena.deinit();
        child_alloc.destroy(self.arena);
    }
};

pub fn loadFromSource(child_allocator: Allocator, source: []const u8) ConfigError!AppConfig {
    var doc = try toml.parse(child_allocator, source);
    defer doc.deinit();

    const arena = try child_allocator.create(std.heap.ArenaAllocator);
    arena.* = std.heap.ArenaAllocator.init(child_allocator);
    errdefer {
        arena.deinit();
        child_allocator.destroy(arena);
    }
    const alloc = arena.allocator();

    const proj_tbl = doc.getTable("project") orelse return ConfigError.MissingProjectSection;
    const name_raw = proj_tbl.getString("name") orelse return ConfigError.MissingProjectName;
    const version_raw = proj_tbl.getString("version") orelse "0.1.0";
    const entry_raw = proj_tbl.getString("entry") orelse "app.zvelte";

    const name = try alloc.dupe(u8, name_raw);
    const version = try alloc.dupe(u8, version_raw);
    const entry = try alloc.dupe(u8, entry_raw);

    // [app]
    var app_mode = Mode.hybrid;
    var app_renderer = Renderer.js;
    var app_target: []const u8 = "web";
    var app_strict = true;

    if (doc.getTable("app")) |tbl| {
        if (tbl.getString("target")) |t| app_target = try alloc.dupe(u8, t);
        if (tbl.getString("mode")) |m| {
            app_mode = Mode.fromString(m) orelse return ConfigError.InvalidMode;
        }
        if (tbl.getString("renderer")) |r| {
            app_renderer = Renderer.fromString(r) orelse return ConfigError.InvalidRenderer;
        }
        if (tbl.getBool("strict")) |s| app_strict = s;
    }

    // [toolchain]
    var tc_zig: []const u8 = "0.17.1";
    var tc_python: []const u8 = "3.15";
    if (doc.getTable("toolchain")) |tbl| {
        if (tbl.getString("zig")) |z| tc_zig = try alloc.dupe(u8, z);
        if (tbl.getString("python")) |p| tc_python = try alloc.dupe(u8, p);
    }

    // [python]
    var py_enabled = true;
    var py_exe: []const u8 = "python3";
    var py_exec: []const u8 = "server";
    var py_zig_accel = true;
    if (doc.getTable("python")) |tbl| {
        if (tbl.getBool("enabled")) |e| py_enabled = e;
        if (tbl.getString("executable")) |e| py_exe = try alloc.dupe(u8, e);
        if (tbl.getString("execution")) |e| py_exec = try alloc.dupe(u8, e);
        if (tbl.getBool("zig_acceleration")) |a| py_zig_accel = a;
    }

    // [zig]
    var zig_enabled = true;
    var zig_exe: []const u8 = "zig";
    var zig_native = true;
    var zig_wasm = true;
    if (doc.getTable("zig")) |tbl| {
        if (tbl.getBool("enabled")) |e| zig_enabled = e;
        if (tbl.getString("executable")) |e| zig_exe = try alloc.dupe(u8, e);
        if (tbl.getBool("native_modules")) |n| zig_native = n;
        if (tbl.getBool("wasm_modules")) |w| zig_wasm = w;
    }

    // [dev]
    var dev_host: []const u8 = "127.0.0.1";
    var dev_port: u16 = 3000;
    var dev_hot = true;
    var dev_open = false;
    if (doc.getTable("dev")) |tbl| {
        if (tbl.getString("host")) |h| dev_host = try alloc.dupe(u8, h);
        if (tbl.getInt("port")) |p| {
            if (p <= 0 or p > 65535) return ConfigError.InvalidPort;
            dev_port = @intCast(p);
        }
        if (tbl.getBool("hot_reload")) |h| dev_hot = h;
        if (tbl.getBool("open_browser")) |o| dev_open = o;
    }

    // [build]
    var b_out: []const u8 = "dist";
    var b_cache: []const u8 = ".zvelte";
    var b_opt = Optimization.debug;
    var b_minify = false;
    var b_maps = true;
    if (doc.getTable("build")) |tbl| {
        if (tbl.getString("output")) |o| b_out = try alloc.dupe(u8, o);
        if (tbl.getString("cache")) |c| b_cache = try alloc.dupe(u8, c);
        if (tbl.getString("optimization")) |opt| {
            b_opt = Optimization.fromString(opt) orelse .debug;
        }
        if (tbl.getBool("minify")) |m| b_minify = m;
        if (tbl.getBool("source_maps")) |s| b_maps = s;
    }

    return AppConfig{
        .child_allocator = child_allocator,
        .arena = arena,
        .project = .{
            .name = name,
            .version = version,
            .entry = entry,
        },
        .app = .{
            .target = app_target,
            .mode = app_mode,
            .renderer = app_renderer,
            .strict = app_strict,
        },
        .toolchain = .{
            .zig = tc_zig,
            .python = tc_python,
        },
        .python = .{
            .enabled = py_enabled,
            .executable = py_exe,
            .execution = py_exec,
            .zig_acceleration = py_zig_accel,
        },
        .zig = .{
            .enabled = zig_enabled,
            .executable = zig_exe,
            .native_modules = zig_native,
            .wasm_modules = zig_wasm,
        },
        .dev = .{
            .host = dev_host,
            .port = dev_port,
            .hot_reload = dev_hot,
            .open_browser = dev_open,
        },
        .build = .{
            .output = b_out,
            .cache = b_cache,
            .optimization = b_opt,
            .minify = b_minify,
            .source_maps = b_maps,
        },
    };
}

test "load default app.toml" {
    const allocator = std.testing.allocator;
    const toml_content =
        \\[project]
        \\name = "my-app"
        \\version = "0.1.0"
        \\entry = "app.zvelte"
        \\
        \\[app]
        \\target = "web"
        \\mode = "hybrid"
        \\renderer = "js"
        \\strict = true
        \\
        \\[dev]
        \\host = "127.0.0.1"
        \\port = 3000
        \\hot_reload = true
    ;

    var config = try loadFromSource(allocator, toml_content);
    defer config.deinit();

    try std.testing.expectEqualStrings("my-app", config.project.name);
    try std.testing.expectEqual(Mode.hybrid, config.app.mode);
    try std.testing.expectEqual(Renderer.js, config.app.renderer);
    try std.testing.expectEqual(@as(u16, 3000), config.dev.port);
    try std.testing.expectEqual(true, config.dev.hot_reload);
}
