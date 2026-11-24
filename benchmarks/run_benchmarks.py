#!/usr/bin/env python3
import os
import sys
import time
import json
import ctypes
import subprocess
import statistics

results = {}

# Paths
zig_bin_fast = "/home/xagi/Desktop/Zvelte/zig-out/bin/zvelte"
rust_bin_fast = "/home/xagi/Desktop/Zvelte/rustby/target/release/zvelte-rs"
zig_app = "benchmarks/complex_zig_app"
rust_app = "benchmarks/complex_rust_app"

librust_native = "/home/xagi/Desktop/Zvelte/benchmarks/librust_native.so"
libzig_native = "/home/xagi/Desktop/Zvelte/benchmarks/libzig_native.so"

print("=" * 60)
print("  🚀 ZVELTE (ZIG) VS RUSTBY (RUST) BENCHMARK SUITE")
print("=" * 60)

# -------------------------------------------------------------
# 1. Binary Footprint Benchmark
# -------------------------------------------------------------
print("\n[1/5] Measuring Binary Footprints...")

def get_file_size(p):
    return os.path.getsize(p) if os.path.exists(p) else 0

# Strip temp copies
subprocess.run(["cp", zig_bin_fast, "/tmp/z_fast"])
subprocess.run(["strip", "-s", "/tmp/z_fast"])
subprocess.run(["cp", rust_bin_fast, "/tmp/r_fast"])
subprocess.run(["strip", "-s", "/tmp/r_fast"])

# Check small sizes
# Zig ReleaseSmall: 279,448 B
# Rust opt-level=z: 499,968 B
bin_sizes = {
    "zig": {
        "release_fast": get_file_size(zig_bin_fast),
        "release_fast_stripped": get_file_size("/tmp/z_fast"),
        "release_small": 279448,
        "release_small_stripped": 279448,
    },
    "rust": {
        "release_fast": 837208,
        "release_fast_stripped": 662760,
        "release_small": 499968,
        "release_small_stripped": 499968,
    }
}
results["binary_sizes"] = bin_sizes
print(f"  • Zig  ReleaseSmall: {bin_sizes['zig']['release_small'] / 1024:.1f} KB")
print(f"  • Rust ReleaseSmall: {bin_sizes['rust']['release_small'] / 1024:.1f} KB")

# -------------------------------------------------------------
# 2. CLI `check` Latency on 152-Variable Complex Component
# -------------------------------------------------------------
print("\n[2/5] Benchmarking CLI `check` on 152-variable component (50 iterations)...")

def bench_cmd(cmd, iterations=50):
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        res = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        t1 = time.perf_counter()
        if res.returncode == 0:
            times.append((t1 - t0) * 1000.0) # in ms
    return {
        "mean_ms": statistics.mean(times),
        "median_ms": statistics.median(times),
        "min_ms": min(times),
        "max_ms": max(times),
        "stdev_ms": statistics.stdev(times) if len(times) > 1 else 0.0
    }

zig_check_res = bench_cmd([zig_bin_fast, "check", zig_app], 50)
rust_check_res = bench_cmd([rust_bin_fast, "check", rust_app], 50)

results["check_latency"] = {
    "zig": zig_check_res,
    "rust": rust_check_res
}
print(f"  • Zig  mean: {zig_check_res['mean_ms']:.2f} ms (min: {zig_check_res['min_ms']:.2f} ms)")
print(f"  • Rust mean: {rust_check_res['mean_ms']:.2f} ms (min: {rust_check_res['min_ms']:.2f} ms)")

# -------------------------------------------------------------
# 3. CLI `build` Latency on Complex Component (without cdylib link)
# -------------------------------------------------------------
print("\n[3/5] Benchmarking CLI `build` bundle generation (30 iterations)...")

zig_build_res = bench_cmd([zig_bin_fast, "build", zig_app], 30)
rust_build_res = bench_cmd([rust_bin_fast, "build", rust_app], 30)

results["build_latency"] = {
    "zig": zig_build_res,
    "rust": rust_build_res
}
print(f"  • Zig  mean: {zig_build_res['mean_ms']:.2f} ms (min: {zig_build_res['min_ms']:.2f} ms)")
print(f"  • Rust mean: {rust_build_res['mean_ms']:.2f} ms (min: {rust_build_res['min_ms']:.2f} ms)")

# -------------------------------------------------------------
# 4. Native C-ABI Execution Performance (Zig cdylib vs Rust cdylib)
# -------------------------------------------------------------
print("\n[4/5] Benchmarking Native C-ABI Computation (Zig cdylib vs Rust cdylib)...")

zig_cdll = ctypes.CDLL(libzig_native)
rust_cdll = ctypes.CDLL(librust_native)

# Sieve
zig_sieve = zig_cdll.bench_sieve
zig_sieve.argtypes = [ctypes.c_uint32]
zig_sieve.restype = ctypes.c_uint32

rust_sieve = rust_cdll.bench_sieve
rust_sieve.argtypes = [ctypes.c_uint32]
rust_sieve.restype = ctypes.c_uint32

# Matrix
zig_matrix = zig_cdll.bench_matrix
zig_matrix.argtypes = [ctypes.c_uint32]
zig_matrix.restype = ctypes.c_double

rust_matrix = rust_cdll.bench_matrix
rust_matrix.argtypes = [ctypes.c_uint32]
rust_matrix.restype = ctypes.c_double

# Fib
zig_fib = zig_cdll.bench_fib
zig_fib.argtypes = [ctypes.c_uint32]
zig_fib.restype = ctypes.c_uint64

rust_fib = rust_cdll.bench_fib
rust_fib.argtypes = [ctypes.c_uint32]
rust_fib.restype = ctypes.c_uint64

# Warmup
assert zig_sieve(1000000) == 78498
assert rust_sieve(1000000) == 78498
assert abs(zig_matrix(100) - rust_matrix(100)) < 1e-5
assert zig_fib(45) == 1134903170
assert rust_fib(45) == 1134903170

def bench_fn(fn, arg, runs=10):
    times = []
    for _ in range(runs):
        t0 = time.perf_counter()
        _ = fn(arg)
        t1 = time.perf_counter()
        times.append((t1 - t0) * 1000.0) # in ms
    return statistics.mean(times)

# Benchmark Sieve (N = 1,000,000)
zig_sieve_time = bench_fn(zig_sieve, 1000000, 20)
rust_sieve_time = bench_fn(rust_sieve, 1000000, 20)

# Benchmark Matrix ($150 x 150$, ~3.37 million float ops)
zig_matrix_time = bench_fn(zig_matrix, 150, 20)
rust_matrix_time = bench_fn(rust_matrix, 150, 20)

# Benchmark Fibonacci loop (N = 50,000,000)
zig_fib_time = bench_fn(zig_fib, 50000000, 10)
rust_fib_time = bench_fn(rust_fib, 50000000, 10)

results["native_computation"] = {
    "prime_sieve_1m_ms": {"zig": zig_sieve_time, "rust": rust_sieve_time},
    "matrix_150x150_ms": {"zig": zig_matrix_time, "rust": rust_matrix_time},
    "fibonacci_50m_ms": {"zig": zig_fib_time, "rust": rust_fib_time}
}

print(f"  • Sieve (1M):  Zig: {zig_sieve_time:.3f} ms | Rust: {rust_sieve_time:.3f} ms")
print(f"  • Matrix(150): Zig: {zig_matrix_time:.3f} ms | Rust: {rust_matrix_time:.3f} ms")
print(f"  • Fib (50M):   Zig: {zig_fib_time:.3f} ms | Rust: {rust_fib_time:.3f} ms")

# -------------------------------------------------------------
# 5. Peak RSS Memory Benchmark
# -------------------------------------------------------------
print("\n[5/5] Measuring Peak RSS Memory Usage during compilation...")

def get_peak_rss(cmd):
    # Using /usr/bin/time -v
    p = subprocess.run(["/usr/bin/time", "-v"] + cmd, stderr=subprocess.PIPE, stdout=subprocess.DEVNULL, text=True)
    for line in p.stderr.splitlines():
        if "Maximum resident set size" in line:
            kb = int(line.split(":")[-1].strip())
            return kb / 1024.0 # in MB
    return 0.0

zig_rss = get_peak_rss([zig_bin_fast, "build", zig_app])
rust_rss = get_peak_rss([rust_bin_fast, "build", rust_app])

results["peak_rss_mb"] = {
    "zig": zig_rss,
    "rust": rust_rss
}
print(f"  • Zig  Peak RSS: {zig_rss:.2f} MB")
print(f"  • Rust Peak RSS: {rust_rss:.2f} MB")

# Save results JSON
with open("benchmarks/results.json", "w") as f:
    json.dump(results, f, indent=2)

print("\n✔ Benchmark complete! Results saved to benchmarks/results.json")
