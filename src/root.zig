const std = @import("std");

pub const toml = @import("config/toml.zig");
pub const config = @import("config/config.zig");
pub const scaffold = @import("cli/scaffold.zig");
pub const ast = @import("source/ast.zig");
pub const token = @import("source/token.zig");
pub const parser = @import("source/parser.zig");
pub const analyzer = @import("semantic/analyzer.zig");
pub const js_codegen = @import("compiler/javascript/js_codegen.zig");
pub const codegen = @import("compiler/codegen.zig");
pub const devserver = @import("devserver/server.zig");
pub const native_compiler = @import("compiler/native/native_compiler.zig");
pub const args = @import("cli/args.zig");

test {
    _ = toml;
    _ = config;
    _ = scaffold;
    _ = parser;
    _ = analyzer;
    _ = js_codegen;
    _ = codegen;
    _ = devserver;
    _ = native_compiler;
}
