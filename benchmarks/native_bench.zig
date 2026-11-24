// native_bench.zig
const std = @import("std");

pub export fn bench_sieve(limit: u32) callconv(.c) u32 {
    const lim = @as(usize, limit);
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    const is_prime = allocator.alloc(bool, lim + 1) catch return 0;
    @memset(is_prime, true);

    var p: usize = 2;
    while (p * p <= lim) : (p += 1) {
        if (is_prime[p]) {
            var i: usize = p * p;
            while (i <= lim) : (i += p) {
                is_prime[i] = false;
            }
        }
    }

    var count: u32 = 0;
    var i: usize = 2;
    while (i <= lim) : (i += 1) {
        if (is_prime[i]) count += 1;
    }
    return count;
}

pub export fn bench_matrix(n: u32) callconv(.c) f64 {
    const size = @as(usize, n);
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    const a = allocator.alloc(f64, size * size) catch return 0.0;
    const b = allocator.alloc(f64, size * size) catch return 0.0;
    const c = allocator.alloc(f64, size * size) catch return 0.0;

    @memset(a, 1.0);
    @memset(b, 2.0);
    @memset(c, 0.0);

    var i: usize = 0;
    while (i < size) : (i += 1) {
        var k: usize = 0;
        while (k < size) : (k += 1) {
            const r = a[i * size + k];
            var j: usize = 0;
            while (j < size) : (j += 1) {
                c[i * size + j] += r * b[k * size + j];
            }
        }
    }

    var sum: f64 = 0.0;
    for (c) |v| {
        sum += v;
    }
    return sum;
}

pub export fn bench_fib(n: u32) callconv(.c) u64 {
    if (n <= 1) return @as(u64, n);
    var a: u64 = 0;
    var b: u64 = 1;
    var i: u32 = 2;
    while (i <= n) : (i += 1) {
        const next = a + b;
        a = b;
        b = next;
    }
    return b;
}
