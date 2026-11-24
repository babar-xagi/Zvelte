const std = @import("std");
const Io = std.Io;
const Dir = std.Io.Dir;

pub const ScaffoldError = error{
    InvalidProjectName,
    DirectoryAlreadyExists,
    DirectoryNotEmpty,
    FileCreationFailed,
    PathTraversalNotAllowed,
} || std.mem.Allocator.Error || std.Io.Dir.CreateDirPathError || std.Io.Dir.OpenError || std.Io.File.OpenError || std.Io.File.Writer.Error;

pub fn validateProjectName(name: []const u8) bool {
    if (name.len == 0 or name.len > 128) return false;
    if (std.mem.eql(u8, name, ".") or std.mem.eql(u8, name, "..")) return false;

    for (name) |c| {
        if (c == 47 or c == 92) return false; // / or \
        if (!((c >= 97 and c <= 122) or (c >= 65 and c <= 90) or (c >= 48 and c <= 57) or c == 45 or c == 95)) {
            return false;
        }
    }
    return true;
}

pub fn getAppTomlTemplate(allocator: std.mem.Allocator, project_name: []const u8) ![]u8 {
    return std.fmt.allocPrint(allocator,
        \\[project]
        \\name = "{s}"
        \\version = "0.1.0"
        \\entry = "app.zvelte"
        \\
        \\[app]
        \\target = "web"
        \\mode = "hybrid"              # "python", "zig", or "hybrid"
        \\renderer = "js"               # "js", "wasm", or later "auto"
        \\strict = true
        \\
        \\[toolchain]
        \\zig = "0.17.1"
        \\python = "3.15"
        \\
        \\[python]
        \\enabled = true
        \\executable = "python3"
        \\execution = "server"          # CPython server-side; not browser Python
        \\zig_acceleration = true
        \\
        \\[zig]
        \\enabled = true
        \\executable = "zig"
        \\native_modules = true
        \\wasm_modules = true
        \\
        \\[dev]
        \\host = "127.0.0.1"
        \\port = 3000
        \\hot_reload = true
        \\open_browser = false
        \\
        \\[build]
        \\output = "dist"
        \\cache = ".zvelte"
        \\optimization = "debug"        # "debug", "release-safe", "release-fast"
        \\minify = false
        \\source_maps = true
        \\
    , .{project_name});
}

pub const APP_ZVELTE_TEMPLATE =
    \\<script lang="python">
    \\name = "Babar"
    \\count = 0
    \\
    \\def greeting(user):
    \\    return f"Hello {user}! Greetings from Python server worker."
    \\</script>
    \\
    \\<main>
    \\    <h1>Hello {name}</h1>
    \\    <p>{greeting(name)}</p>
    \\
    \\    <div class="card">
    \\        <button click={count++}>
    \\            Clicked {count} times
    \\        </button>
    \\        <button click={count = 0}>
    \\            Reset
    \\        </button>
    \\    </div>
    \\</main>
    \\
    \\<style>
    \\main {
    \\    font-family: system-ui, -apple-system, sans-serif;
    \\    max-width: 36rem;
    \\    margin: 4rem auto;
    \\    text-align: center;
    \\    padding: 2.5rem;
    \\    background: #ffffff;
    \\    border-radius: 12px;
    \\    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
    \\}
    \\.card {
    \\    margin-top: 1.5rem;
    \\    display: flex;
    \\    justify-content: center;
    \\    gap: 0.75rem;
    \\}
    \\button {
    \\    padding: 0.6rem 1.4rem;
    \\    border-radius: 8px;
    \\    border: 1px solid #0070f3;
    \\    background: #0070f3;
    \\    color: white;
    \\    font-size: 1rem;
    \\    font-weight: 500;
    \\    cursor: pointer;
    \\}
    \\button:hover {
    \\    background: #005bb5;
    \\}
    \\</style>
    \\
;

pub fn createProject(allocator: std.mem.Allocator, io: Io, project_name: []const u8, base_dir: Dir) ScaffoldError!void {
    if (!validateProjectName(project_name)) {
        return ScaffoldError.InvalidProjectName;
    }

    if (base_dir.openDir(io, project_name, .{})) |existing_dir| {
        var dir_to_close = existing_dir;
        defer dir_to_close.close(io);
        return ScaffoldError.DirectoryAlreadyExists;
    } else |_| {}

    try base_dir.createDirPath(io, project_name);

    var proj_dir = try base_dir.openDir(io, project_name, .{});
    defer proj_dir.close(io);

    const toml_content = try getAppTomlTemplate(allocator, project_name);
    defer allocator.free(toml_content);

    var toml_file = try proj_dir.createFile(io, "app.toml", .{});
    defer toml_file.close(io);
    try toml_file.writeStreamingAll(io, toml_content);

    var zvelte_file = try proj_dir.createFile(io, "app.zvelte", .{});
    defer zvelte_file.close(io);
    try zvelte_file.writeStreamingAll(io, APP_ZVELTE_TEMPLATE);
}

test "scaffold creates exactly two files" {
    const allocator = std.testing.allocator;
    const io = std.testing.io;
    const cwd = Dir.cwd();

    const test_proj = "test_two_files_app";
    defer {
        if (cwd.openDir(io, test_proj, .{})) |d| {
            var dir = d;
            cwd.deleteFile(io, "test_two_files_app/app.toml") catch {};
            cwd.deleteFile(io, "test_two_files_app/app.zvelte") catch {};
            dir.close(io);
            cwd.deleteDir(io, test_proj) catch {};
        } else |_| {}
    }

    try createProject(allocator, io, test_proj, cwd);

    var proj_dir = try cwd.openDir(io, test_proj, .{});
    defer proj_dir.close(io);

    var toml_file = try proj_dir.openFile(io, "app.toml", .{});
    toml_file.close(io);

    var zvelte_file = try proj_dir.openFile(io, "app.zvelte", .{});
    zvelte_file.close(io);

    const err = createProject(allocator, io, test_proj, cwd);
    try std.testing.expectError(ScaffoldError.DirectoryAlreadyExists, err);

    try std.testing.expect(!validateProjectName("../invalid"));
    try std.testing.expect(!validateProjectName("bad/name"));
    try std.testing.expect(!validateProjectName(""));
    try std.testing.expect(validateProjectName("my-zvelte-app"));
}

test "prove strictly two files exist and no third file" {
    const allocator = std.testing.allocator;
    const io = std.testing.io;
    const cwd = Dir.cwd();

    const test_proj = "test_strict_two_files";
    defer {
        if (cwd.openDir(io, test_proj, .{})) |d| {
            var dir = d;
            cwd.deleteFile(io, "test_strict_two_files/app.toml") catch {};
            cwd.deleteFile(io, "test_strict_two_files/app.zvelte") catch {};
            dir.close(io);
            cwd.deleteDir(io, test_proj) catch {};
        } else |_| {}
    }

    try createProject(allocator, io, test_proj, cwd);

    var proj_dir = try cwd.openDir(io, test_proj, .{ .iterate = true });
    defer proj_dir.close(io);

    var iter = proj_dir.iterate();
    var file_count: usize = 0;
    var found_toml = false;
    var found_zvelte = false;

    while (try iter.next(io)) |entry| {
        file_count += 1;
        if (std.mem.eql(u8, entry.name, "app.toml")) {
            found_toml = true;
        } else if (std.mem.eql(u8, entry.name, "app.zvelte")) {
            found_zvelte = true;
        } else {
            // Any third file is a violation of the contract!
            return error.UnexpectedThirdFile;
        }
    }

    try std.testing.expectEqual(@as(usize, 2), file_count);
    try std.testing.expect(found_toml);
    try std.testing.expect(found_zvelte);
}
