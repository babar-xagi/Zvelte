const std = @import("std");
const Allocator = std.mem.Allocator;
const Io = std.Io;
const Dir = std.Io.Dir;

const scaffold = @import("scaffold.zig");
const config = @import("../config/config.zig");
const parser = @import("../source/parser.zig");
const analyzer = @import("../semantic/analyzer.zig");
const codegen = @import("../compiler/codegen.zig");
const devserver = @import("../devserver/server.zig");

pub const VERSION = "0.0.3";

pub fn printHelp() void {
    std.debug.print(
        \\Zvelte — A Zig-powered multi-language reactive web framework (v{s})
        \\
        \\Usage:
        \\  zvelte <command> [arguments] [options]
        \\
        \\Commands:
        \\  new <name>      Create a new Zvelte project (exactly app.toml + app.zvelte)
        \\  check [path]    Validate config, parse source, and run semantic checks
        \\  build [path]    Compile production web assets into dist/
        \\  dev [path]      Start the local development server with file watching
        \\  clean [path]    Remove only Zvelte-generated output and cache directories
        \\
        \\Options:
        \\  --version, -v   Print Zvelte framework version
        \\  --help, -h      Display this help menu
        \\
    , .{VERSION});
}

pub fn printVersion() void {
    std.debug.print("zvelte version {s}\n", .{VERSION});
}

pub fn execute(allocator: Allocator, io: Io, args: []const []const u8) !void {
    if (args.len <= 1) {
        printHelp();
        return;
    }

    const command = args[1];

    if (std.mem.eql(u8, command, "--version") or std.mem.eql(u8, command, "-v") or std.mem.eql(u8, command, "version")) {
        printVersion();
        return;
    }

    if (std.mem.eql(u8, command, "--help") or std.mem.eql(u8, command, "-h") or std.mem.eql(u8, command, "help")) {
        printHelp();
        return;
    }

    if (std.mem.eql(u8, command, "new")) {
        if (args.len < 3) {
            std.debug.print("\x1b[31merror:\x1b[0m 'zvelte new' requires a project name.\nUsage: zvelte new <project-name>\n", .{});
            return;
        }
        const proj_name = args[2];
        const cwd = Dir.cwd();
        scaffold.createProject(allocator, io, proj_name, cwd) catch |err| {
            switch (err) {
                error.DirectoryAlreadyExists => {
                    std.debug.print("\x1b[31merror:\x1b[0m directory '{s}' already exists. Refusing to overwrite.\n", .{proj_name});
                },
                error.InvalidProjectName => {
                    std.debug.print("\x1b[31merror:\x1b[0m '{s}' is not a valid project name.\n", .{proj_name});
                },
                else => {
                    std.debug.print("\x1b[31merror:\x1b[0m failed to create project: {s}\n", .{@errorName(err)});
                },
            }
            return err;
        };

        std.debug.print("\n\x1b[32m✔\x1b[0m Created Zvelte project \x1b[1m{s}\x1b[0m\n", .{proj_name});
        std.debug.print("  ├── app.toml\n", .{});
        std.debug.print("  └── app.zvelte\n\n", .{});
        std.debug.print("Next steps:\n", .{});
        std.debug.print("  cd {s}\n", .{proj_name});
        std.debug.print("  zvelte dev\n\n", .{});
        return;
    }

    // Target directory (default: current directory)
    const target_path = if (args.len >= 3) args[2] else ".";
    const cwd = Dir.cwd();
    var project_dir = cwd.openDir(io, target_path, .{}) catch |err| {
        std.debug.print("\x1b[31merror:\x1b[0m failed to open project directory '{s}': {s}\n", .{ target_path, @errorName(err) });
        return err;
    };
    defer project_dir.close(io);

    if (std.mem.eql(u8, command, "check")) {
        // Read & validate app.toml
        const toml_source = project_dir.readFileAlloc(io, "app.toml", allocator, .unlimited) catch |err| {
            std.debug.print("\x1b[31merror:\x1b[0m failed to read app.toml: {s}\n", .{@errorName(err)});
            return err;
        };
        defer allocator.free(toml_source);

        var cfg = config.loadFromSource(allocator, toml_source) catch |err| {
            std.debug.print("\x1b[31merror:\x1b[0m invalid configuration in app.toml: {s}\n", .{@errorName(err)});
            return err;
        };
        defer cfg.deinit();

        // Read entry .zvelte
        const zvelte_source = project_dir.readFileAlloc(io, cfg.project.entry, allocator, .unlimited) catch |err| {
            std.debug.print("\x1b[31merror:\x1b[0m failed to read {s}: {s}\n", .{ cfg.project.entry, @errorName(err) });
            return err;
        };
        defer allocator.free(zvelte_source);

        // Parse component
        var comp = parser.parseComponent(allocator, zvelte_source) catch |err| {
            std.debug.print("\x1b[31mparse error:\x1b[0m in {s}: {s}\n", .{ cfg.project.entry, @errorName(err) });
            return err;
        };
        defer comp.deinit(allocator);

        // Semantic analysis
        var sem_model = analyzer.analyze(allocator, comp, cfg) catch |err| {
            std.debug.print("\x1b[31msemantic error:\x1b[0m in {s}: {s}\n", .{ cfg.project.entry, @errorName(err) });
            return err;
        };
        defer sem_model.deinit(allocator);

        std.debug.print("\x1b[32m✔\x1b[0m Check passed: '{s}' (mode: {s}, renderer: {s}, {d} state vars, {d} scripts)\n", .{
            cfg.project.name,
            @tagName(cfg.app.mode),
            @tagName(cfg.app.renderer),
            comp.states.len,
            comp.scripts.len,
        });
        return;
    }

    if (std.mem.eql(u8, command, "build")) {
        std.debug.print("Building Zvelte project for production...\n", .{});
        const res = codegen.buildProject(allocator, io, project_dir, "dist") catch |err| {
            std.debug.print("\x1b[31mbuild error:\x1b[0m {s}\n", .{@errorName(err)});
            return err;
        };

        std.debug.print("\x1b[32m✔\x1b[0m Production build complete in \x1b[1m{s}/\x1b[0m\n", .{res.output_dir});
        std.debug.print("  ├── index.html ({d} bytes)\n", .{res.html_bytes});
        if (res.worker_bytes > 0) {
            std.debug.print("  ├── app.js ({d} bytes)\n", .{res.js_bytes});
            std.debug.print("  └── worker.py ({d} bytes) [Python Server RPC]\n", .{res.worker_bytes});
        } else {
            std.debug.print("  └── app.js ({d} bytes)\n", .{res.js_bytes});
        }
        return;
    }

    if (std.mem.eql(u8, command, "dev")) {
        // Load config for dev host & port
        const toml_source = project_dir.readFileAlloc(io, "app.toml", allocator, .unlimited) catch |err| {
            std.debug.print("\x1b[31merror:\x1b[0m failed to read app.toml: {s}\n", .{@errorName(err)});
            return err;
        };
        defer allocator.free(toml_source);

        var cfg = config.loadFromSource(allocator, toml_source) catch |err| {
            std.debug.print("\x1b[31merror:\x1b[0m invalid configuration in app.toml: {s}\n", .{@errorName(err)});
            return err;
        };
        defer cfg.deinit();

        var server = try devserver.DevServer.init(allocator, io, project_dir, cfg.dev.host, cfg.dev.port);
        defer server.deinit();

        try server.serve(null);
        return;
    }

    if (std.mem.eql(u8, command, "clean")) {
        std.debug.print("Cleaning Zvelte build artifacts and cache...\n", .{});
        // Remove only Zvelte generated output
        project_dir.deleteFile(io, "dist/index.html") catch {};
        project_dir.deleteFile(io, "dist/app.js") catch {};
        project_dir.deleteFile(io, "dist/worker.py") catch {};
        project_dir.deleteDir(io, "dist") catch {};

        project_dir.deleteFile(io, ".zvelte/dev/index.html") catch {};
        project_dir.deleteFile(io, ".zvelte/dev/app.js") catch {};
        project_dir.deleteFile(io, ".zvelte/dev/worker.py") catch {};
        project_dir.deleteDir(io, ".zvelte/dev") catch {};
        project_dir.deleteDir(io, ".zvelte") catch {};

        std.debug.print("\x1b[32m✔\x1b[0m Clean complete. User source files preserved.\n", .{});
        return;
    }

    std.debug.print("\x1b[31merror:\x1b[0m unknown command '{s}'. Run 'zvelte --help' for available commands.\n", .{command});
}
