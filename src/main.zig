const std = @import("std");
const zvelte = @import("zvelte");

pub fn main(init: std.process.Init) !void {
    const arena = init.arena.allocator();
    const args = try init.minimal.args.toSlice(arena);
    const io = init.io;

    try zvelte.args.execute(arena, io, args);
}
