const std = @import("std");
const Allocator = std.mem.Allocator;
const Io = std.Io;
const Dir = std.Io.Dir;

const config = @import("../config/config.zig");
const parser = @import("../source/parser.zig");
const analyzer = @import("../semantic/analyzer.zig");
const js_codegen = @import("javascript/js_codegen.zig");

pub const BuildResult = struct {
    output_dir: []const u8,
    html_bytes: usize,
    js_bytes: usize,
};

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

    return BuildResult{
        .output_dir = output_dir_name,
        .html_bytes = out.html.len,
        .js_bytes = out.js.len,
    };
}
