const std = @import("std");
const Allocator = std.mem.Allocator;
const Io = std.Io;
const Dir = std.Io.Dir;
const linux = std.os.linux;

const codegen = @import("../compiler/codegen.zig");

pub const ServerError = error{
    SocketCreationFailed,
    BindFailed,
    ListenFailed,
    AcceptFailed,
    BuildFailed,
} || Allocator.Error || std.Io.Dir.CreateDirPathError || std.Io.Dir.OpenError || std.Io.File.OpenError;

fn isError(res: usize) bool {
    const s: isize = @bitCast(res);
    return s < 0;
}

pub const DevServer = struct {
    allocator: Allocator,
    io: Io,
    project_dir: Dir,
    host: []const u8,
    port: u16,
    socket_fd: linux.fd_t,
    python_child: ?std.process.Child = null,

    pub fn init(allocator: Allocator, io: Io, project_dir: Dir, host: []const u8, port: u16) ServerError!DevServer {
        const fd_usize = linux.socket(linux.AF.INET, linux.SOCK.STREAM, 0);
        if (isError(fd_usize)) {
            return ServerError.SocketCreationFailed;
        }
        const fd: linux.fd_t = @intCast(fd_usize);
        errdefer _ = linux.close(fd);

        // SO_REUSEADDR
        const optval: c_int = 1;
        _ = linux.setsockopt(fd, linux.SOL.SOCKET, linux.SO.REUSEADDR, @ptrCast(&optval), @sizeOf(c_int));

        var addr: linux.sockaddr.in = .{
            .port = std.mem.nativeToBig(u16, port),
            .addr = std.mem.nativeToBig(u32, 0x7f000001), // 127.0.0.1
        };

        const bind_res = linux.bind(fd, @ptrCast(&addr), @sizeOf(linux.sockaddr.in));
        if (isError(bind_res)) {
            return ServerError.BindFailed;
        }

        const listen_res = linux.listen(fd, 128);
        if (isError(listen_res)) {
            return ServerError.ListenFailed;
        }

        return DevServer{
            .allocator = allocator,
            .io = io,
            .project_dir = project_dir,
            .host = host,
            .port = port,
            .socket_fd = fd,
            .python_child = null,
        };
    }

    pub fn deinit(self: *DevServer) void {
        if (self.python_child) |*child| {
            child.kill(self.io);
            _ = child.wait(self.io) catch {};
            self.python_child = null;
        }
        _ = linux.close(self.socket_fd);
    }

    pub fn serve(self: *DevServer, max_requests: ?usize) !void {
        // 1. Initial build to .zvelte/dev
        _ = try codegen.buildProject(self.allocator, self.io, self.project_dir, ".zvelte/dev");

        // 2. Check and spawn Python worker if .zvelte/dev/worker.py exists
        var dev_dir = self.project_dir.openDir(self.io, ".zvelte/dev", .{}) catch null;
        if (dev_dir) |*d| {
            defer d.close(self.io);
            if (d.openFile(self.io, "worker.py", .{})) |wf| {
                var worker_file = wf;
                worker_file.close(self.io);

                const worker_path = ".zvelte/dev/worker.py";
                const child = std.process.spawn(self.io, .{
                    .argv = &.{ "python3", "-u", worker_path },
                    .cwd = .{ .dir = self.project_dir },
                    .stdin = .pipe,
                    .stdout = .pipe,
                }) catch |err| blk: {
                    std.debug.print("  \x1b[33m!\x1b[0m Failed to spawn Python worker: {s}\n", .{@errorName(err)});
                    break :blk null;
                };
                self.python_child = child;
                if (self.python_child != null) {
                    std.debug.print("  \x1b[32m✔\x1b[0m Python server worker supervisor online\n", .{});
                }
            } else |_| {}
        }

        std.debug.print("\n  \x1b[32m✔\x1b[0m Zvelte dev server running at \x1b[1mhttp://{s}:{d}/\x1b[0m\n\n", .{
            self.host,
            self.port,
        });

        var req_count: usize = 0;
        while (max_requests == null or req_count < max_requests.?) {
            var client_addr: linux.sockaddr = undefined;
            var client_len: linux.socklen_t = @sizeOf(linux.sockaddr);

            const client_res = linux.accept(self.socket_fd, &client_addr, &client_len);
            if (isError(client_res)) continue;
            const client_fd: linux.fd_t = @intCast(client_res);
            defer _ = linux.close(client_fd);

            req_count += 1;

            var req_list: std.ArrayList(u8) = .empty;
            defer req_list.deinit(self.allocator);

            var buf: [4096]u8 = undefined;
            const read_res = linux.read(client_fd, &buf, buf.len);
            if (isError(read_res) or read_res == 0) continue;
            try req_list.appendSlice(self.allocator, buf[0..read_res]);

            // If request has Content-Length, ensure full body is read
            const initial_str = req_list.items;
            if (std.mem.indexOf(u8, initial_str, "Content-Length: ")) |cl_idx| {
                const after = initial_str[cl_idx + 16 ..];
                var cl_end: usize = 0;
                while (cl_end < after.len and after[cl_end] >= '0' and after[cl_end] <= '9') {
                    cl_end += 1;
                }
                if (cl_end > 0) {
                    const content_len = std.fmt.parseInt(usize, after[0..cl_end], 10) catch 0;
                    const body_start_opt = if (std.mem.indexOf(u8, initial_str, "\r\n\r\n")) |idx|
                        idx + 4
                    else if (std.mem.indexOf(u8, initial_str, "\n\n")) |idx|
                        idx + 2
                    else
                        null;

                    if (body_start_opt) |body_start| {
                        const current_body_len = initial_str.len - body_start;
                        if (current_body_len < content_len) {
                            var remaining = content_len - current_body_len;
                            while (remaining > 0) {
                                const n = linux.read(client_fd, &buf, @min(buf.len, remaining));
                                if (isError(n) or n == 0) break;
                                try req_list.appendSlice(self.allocator, buf[0..n]);
                                remaining -= n;
                            }
                        }
                    }
                }
            }

            try self.handleRequest(client_fd, req_list.items);
        }
    }

    fn handleRequest(self: *DevServer, client_fd: linux.fd_t, req_str: []const u8) !void {
        // Handle CORS preflight
        if (std.mem.startsWith(u8, req_str, "OPTIONS ")) {
            const cors = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n";
            _ = linux.write(client_fd, cors, cors.len);
            return;
        }

        // Handle Python Server RPC Bridge
        if (std.mem.startsWith(u8, req_str, "POST /api/rpc")) {
            if (self.python_child == null) {
                const err_body = "{\"error\": \"Python server worker is not running\"}";
                const header = try std.fmt.allocPrint(self.allocator,
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {d}\r\nConnection: close\r\n\r\n",
                    .{err_body.len},
                );
                defer self.allocator.free(header);
                _ = linux.write(client_fd, header.ptr, header.len);
                _ = linux.write(client_fd, err_body.ptr, err_body.len);
                return;
            }

            const body_start = if (std.mem.indexOf(u8, req_str, "\r\n\r\n")) |idx|
                idx + 4
            else if (std.mem.indexOf(u8, req_str, "\n\n")) |idx|
                idx + 2
            else
                req_str.len;

            const req_body = std.mem.trim(u8, req_str[body_start..], " \r\n\t");

            var child = &self.python_child.?;
            try child.stdin.?.writeStreamingAll(self.io, req_body);
            try child.stdin.?.writeStreamingAll(self.io, "\n");

            var resp_list: std.ArrayList(u8) = .empty;
            defer resp_list.deinit(self.allocator);

            var byte_buf: [1]u8 = undefined;
            while (true) {
                const n = child.stdout.?.readStreaming(self.io, &.{&byte_buf}) catch 0;
                if (n == 0) break;
                if (byte_buf[0] == '\n') break;
                try resp_list.append(self.allocator, byte_buf[0]);
            }

            const resp_line = std.mem.trim(u8, resp_list.items, " \r\n");
            const resp_body = if (resp_line.len > 0) resp_line else "{\"error\": \"Empty response from Python worker\"}";

            const header = try std.fmt.allocPrint(self.allocator,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {d}\r\nConnection: close\r\n\r\n",
                .{resp_body.len},
            );
            defer self.allocator.free(header);

            _ = linux.write(client_fd, header.ptr, header.len);
            _ = linux.write(client_fd, resp_body.ptr, resp_body.len);
            return;
        }

        var path: []const u8 = "/";
        if (std.mem.startsWith(u8, req_str, "GET ")) {
            const start: usize = 4;
            var end: usize = start;
            while (end < req_str.len and req_str[end] != ' ' and req_str[end] != '\r' and req_str[end] != '\n') {
                end += 1;
            }
            path = req_str[start..end];
        }

        var dev_dir = try self.project_dir.openDir(self.io, ".zvelte/dev", .{});
        defer dev_dir.close(self.io);

        if (std.mem.eql(u8, path, "/") or std.mem.eql(u8, path, "/index.html")) {
            const content = dev_dir.readFileAlloc(self.io, "index.html", self.allocator, .unlimited) catch {
                _ = linux.write(client_fd, "HTTP/1.1 500 Internal Error\r\n\r\n", 31);
                return;
            };
            defer self.allocator.free(content);

            const header = try std.fmt.allocPrint(self.allocator,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {d}\r\nConnection: close\r\n\r\n",
                .{content.len},
            );
            defer self.allocator.free(header);

            _ = linux.write(client_fd, header.ptr, header.len);
            _ = linux.write(client_fd, content.ptr, content.len);
        } else if (std.mem.eql(u8, path, "/app.js")) {
            const content = dev_dir.readFileAlloc(self.io, "app.js", self.allocator, .unlimited) catch {
                _ = linux.write(client_fd, "HTTP/1.1 500 Internal Error\r\n\r\n", 31);
                return;
            };
            defer self.allocator.free(content);

            const header = try std.fmt.allocPrint(self.allocator,
                "HTTP/1.1 200 OK\r\nContent-Type: application/javascript; charset=utf-8\r\nContent-Length: {d}\r\nConnection: close\r\n\r\n",
                .{content.len},
            );
            defer self.allocator.free(header);

            _ = linux.write(client_fd, header.ptr, header.len);
            _ = linux.write(client_fd, content.ptr, content.len);
        } else {
            const not_found = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found";
            _ = linux.write(client_fd, not_found.ptr, not_found.len);
        }
    }
};

test "dev server binds and initializes" {
    const allocator = std.testing.allocator;
    const io = std.testing.io;
    const cwd = Dir.cwd();

    var server = try DevServer.init(allocator, io, cwd, "127.0.0.1", 3099);
    defer server.deinit();

    try std.testing.expect(server.socket_fd > 0);
}
