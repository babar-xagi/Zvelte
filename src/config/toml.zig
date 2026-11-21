const std = @import("std");
const Allocator = std.mem.Allocator;

pub const ValueType = enum {
    string,
    integer,
    boolean,
    float,
};

pub const Value = union(ValueType) {
    string: []const u8,
    integer: i64,
    boolean: bool,
    float: f64,

    pub fn asString(self: Value) ?[]const u8 {
        return switch (self) {
            .string => |s| s,
            else => null,
        };
    }

    pub fn asInt(self: Value) ?i64 {
        return switch (self) {
            .integer => |i| i,
            else => null,
        };
    }

    pub fn asBool(self: Value) ?bool {
        return switch (self) {
            .boolean => |b| b,
            else => null,
        };
    }
};

pub const Table = struct {
    name: []const u8,
    entries: std.StringHashMap(Value),

    pub fn init(allocator: Allocator, name: []const u8) Table {
        return .{
            .name = name,
            .entries = std.StringHashMap(Value).init(allocator),
        };
    }

    pub fn deinit(self: *Table) void {
        self.entries.deinit();
    }

    pub fn get(self: *const Table, key: []const u8) ?Value {
        return self.entries.get(key);
    }

    pub fn getString(self: *const Table, key: []const u8) ?[]const u8 {
        if (self.get(key)) |v| {
            return v.asString();
        }
        return null;
    }

    pub fn getInt(self: *const Table, key: []const u8) ?i64 {
        if (self.get(key)) |v| {
            return v.asInt();
        }
        return null;
    }

    pub fn getBool(self: *const Table, key: []const u8) ?bool {
        if (self.get(key)) |v| {
            return v.asBool();
        }
        return null;
    }
};

pub const TomlDoc = struct {
    child_allocator: Allocator,
    arena: *std.heap.ArenaAllocator,
    root: Table,
    tables: std.StringHashMap(Table),

    pub fn init(child_allocator: Allocator) !TomlDoc {
        const arena = try child_allocator.create(std.heap.ArenaAllocator);
        arena.* = std.heap.ArenaAllocator.init(child_allocator);
        const allocator = arena.allocator();
        return .{
            .child_allocator = child_allocator,
            .arena = arena,
            .root = Table.init(allocator, ""),
            .tables = std.StringHashMap(Table).init(allocator),
        };
    }

    pub fn deinit(self: *TomlDoc) void {
        const child_alloc = self.child_allocator;
        self.arena.deinit();
        child_alloc.destroy(self.arena);
    }

    pub fn getTable(self: *const TomlDoc, name: []const u8) ?*const Table {
        if (name.len == 0) return &self.root;
        return self.tables.getPtr(name);
    }
};

pub const TomlError = error{
    InvalidSyntax,
    UnexpectedChar,
    UnterminatedString,
    InvalidNumber,
    OutOfMemory,
};

pub fn parse(child_allocator: Allocator, source: []const u8) TomlError!TomlDoc {
    var doc = try TomlDoc.init(child_allocator);
    errdefer doc.deinit();
    const allocator = doc.arena.allocator();

    var current_table: *Table = &doc.root;
    var i: usize = 0;

    while (i < source.len) {
        while (i < source.len and (source[i] == 32 or source[i] == 9 or source[i] == 13)) {
            i += 1;
        }

        if (i >= source.len) break;

        if (source[i] == 10) {
            i += 1;
            continue;
        }

        if (source[i] == 35) { // #
            while (i < source.len and source[i] != 10) {
                i += 1;
            }
            continue;
        }

        if (source[i] == 91) { // [
            i += 1;
            const start = i;
            while (i < source.len and source[i] != 93 and source[i] != 10) {
                i += 1;
            }
            if (i >= source.len or source[i] != 93) {
                return TomlError.InvalidSyntax;
            }
            const sec_name = std.mem.trim(u8, source[start..i], " \t\r");
            i += 1;

            while (i < source.len and (source[i] == 32 or source[i] == 9 or source[i] == 13)) {
                i += 1;
            }
            if (i < source.len and source[i] == 35) {
                while (i < source.len and source[i] != 10) i += 1;
            }
            if (i < source.len and source[i] == 10) {
                i += 1;
            }

            const table_entry = try doc.tables.getOrPut(sec_name);
            if (!table_entry.found_existing) {
                table_entry.value_ptr.* = Table.init(allocator, sec_name);
            }
            current_table = table_entry.value_ptr;
            continue;
        }

        const key_start = i;
        while (i < source.len and source[i] != 61 and source[i] != 10 and source[i] != 35) { // = \n #
            i += 1;
        }
        if (i >= source.len or source[i] != 61) {
            return TomlError.InvalidSyntax;
        }
        const key = std.mem.trim(u8, source[key_start..i], " \t\r");
        if (key.len == 0) return TomlError.InvalidSyntax;
        i += 1;

        while (i < source.len and (source[i] == 32 or source[i] == 9 or source[i] == 13)) {
            i += 1;
        }

        if (i >= source.len or source[i] == 10) {
            return TomlError.InvalidSyntax;
        }

        var val: Value = undefined;
        if (source[i] == 34) { // "
            i += 1;
            var str_buf: std.ArrayList(u8) = .empty;

            while (i < source.len and source[i] != 34) {
                if (source[i] == 92) { // backslash
                    i += 1;
                    if (i >= source.len) return TomlError.UnterminatedString;
                    switch (source[i]) {
                        110 => try str_buf.append(allocator, 10), // \n
                        116 => try str_buf.append(allocator, 9),  // \t
                        114 => try str_buf.append(allocator, 13), // \r
                        92 => try str_buf.append(allocator, 92),  // \\
                        34 => try str_buf.append(allocator, 34),  // \"
                        else => try str_buf.append(allocator, source[i]),
                    }
                } else {
                    try str_buf.append(allocator, source[i]);
                }
                i += 1;
            }
            if (i >= source.len or source[i] != 34) {
                return TomlError.UnterminatedString;
            }
            i += 1;
            val = .{ .string = try str_buf.toOwnedSlice(allocator) };
        } else if (std.mem.startsWith(u8, source[i..], "true")) {
            val = .{ .boolean = true };
            i += 4;
        } else if (std.mem.startsWith(u8, source[i..], "false")) {
            val = .{ .boolean = false };
            i += 5;
        } else {
            const num_start = i;
            var is_float = false;
            if (source[i] == 45 or source[i] == 43) { // - or +
                i += 1;
            }
            while (i < source.len and ((source[i] >= 48 and source[i] <= 57) or source[i] == 46 or source[i] == 95)) {
                if (source[i] == 46) is_float = true;
                i += 1;
            }
            const raw_num = source[num_start..i];
            if (raw_num.len == 0) return TomlError.InvalidNumber;

            if (is_float) {
                const f = std.fmt.parseFloat(f64, raw_num) catch return TomlError.InvalidNumber;
                val = .{ .float = f };
            } else {
                const num = std.fmt.parseInt(i64, raw_num, 10) catch return TomlError.InvalidNumber;
                val = .{ .integer = num };
            }
        }

        try current_table.entries.put(key, val);

        while (i < source.len and (source[i] == 32 or source[i] == 9 or source[i] == 13)) {
            i += 1;
        }
        if (i < source.len and source[i] == 35) {
            while (i < source.len and source[i] != 10) i += 1;
        }
        if (i < source.len and source[i] == 10) {
            i += 1;
        }
    }

    return doc;
}

test "parse sample toml" {
    const allocator = std.testing.allocator;
    const sample =
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
    var doc = try parse(allocator, sample);
    defer doc.deinit();

    const proj = doc.getTable("project");
    try std.testing.expect(proj != null);
    try std.testing.expectEqualStrings("my-app", proj.?.getString("name").?);

    const app = doc.getTable("app");
    try std.testing.expect(app != null);
    try std.testing.expectEqualStrings("hybrid", app.?.getString("mode").?);
    try std.testing.expectEqual(true, app.?.getBool("strict").?);

    const dev = doc.getTable("dev");
    try std.testing.expect(dev != null);
    try std.testing.expectEqual(@as(i64, 3000), dev.?.getInt("port").?);
}
