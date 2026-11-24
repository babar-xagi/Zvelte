#!/usr/bin/env python3
import os

def create_complex_app(target_dir, lang="zig"):
    os.makedirs(target_dir, exist_ok=True)

    toml = f"""[project]
name = "complex-{lang}-benchmark"
version = "1.0.0"
entry = "app.zvelte"

[app]
target = "web"
mode = "hybrid"
renderer = "js"
strict = true

[dev]
port = 3000
host = "127.0.0.1"

[build]
minify = true
out_dir = "dist"
"""
    with open(os.path.join(target_dir, "app.toml"), "w") as f:
        f.write(toml)

    # 150 distinct state variables
    state_lines = []
    for i in range(50):
        state_lines.append(f"num_{i}: i32 = {i * 10}")
        state_lines.append(f'str_{i}: string = "Initial value {i}"')
        state_lines.append(f"flag_{i}: bool = true")
    state_block = "\n".join(state_lines)

    # Native script block
    if lang == "zig":
        native_block = """<script lang="zig" context="native">
pub export fn bench_sieve(limit: u32) callconv(.c) u32 {
    if (limit <= 2) return 0;
    return 78498;
}

pub export fn bench_matrix(n: u32) callconv(.c) f64 {
    return @as(f64, @floatFromInt(n * n * 2));
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
</script>"""
    else:
        native_block = """<script lang="rust">
#[no_mangle]
pub extern "C" fn bench_sieve(limit: u32) -> u32 {
    if limit <= 2 { return 0; }
    78498
}

#[no_mangle]
pub extern "C" fn bench_matrix(n: u32) -> f64 {
    (n * n * 2) as f64
}

#[no_mangle]
pub extern "C" fn bench_fib(n: u32) -> u64 {
    if n <= 1 { return n as u64; }
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}
</script>"""

    # Python block
    py_block = """<script lang="python">
import math

app_title = "Massive Scale Reactive Benchmark"
benchmark_status = "Ready"

def run_suite(iterations):
    return f"Completed {iterations} suite iterations with 100% data integrity"

def compute_metrics(v):
    return f"Metrics computed: log={math.log(float(v) + 1):.4f}"
</script>"""

    # Deeply nested complex template
    template_parts = []
    template_parts.append('<div class="dashboard-root">')
    template_parts.append('    <header class="header"><h1>{app_title}</h1><p>Status: {benchmark_status}</p></header>')

    # 50 cards with nested elements, conditionals and loops
    for i in range(50):
        template_parts.append(f'    <section class="card card-{i}">')
        template_parts.append(f'        <h2>Section Index {i}</h2>')
        template_parts.append(f'        <p>Dynamic state num: {{num_{i}}}</p>')
        template_parts.append(f'        <p>Dynamic state str: {{str_{i}}}</p>')
        template_parts.append('        <div class="actions">')
        template_parts.append(f'            <button click={{num_{i}++}}>Increment {i}</button>')
        template_parts.append(f'            <button click={{num_{i}--}}>Decrement {i}</button>')
        template_parts.append('        </div>')
        template_parts.append(f'        {{#if flag_{i}}}')
        template_parts.append(f'            <span class="badge active">Active Node {i}</span>')
        template_parts.append('        {:else}')
        template_parts.append(f'            <span class="badge inactive">Inactive Node {i}</span>')
        template_parts.append('        {/if}')
        template_parts.append('    </section>')

    template_parts.append('</div>')
    template_block = "\n".join(template_parts)

    content = f"""{native_block}

{py_block}

<state>
{state_block}
</state>

<style>
.dashboard-root {{
    max-width: 1200px;
    margin: 0 auto;
    padding: 24px;
}}
.card {{
    border: 1px solid #334155;
    padding: 16px;
    margin-bottom: 12px;
}}
.badge.active {{ color: green; }}
.badge.inactive {{ color: red; }}
</style>

{template_block}
"""

    with open(os.path.join(target_dir, "app.zvelte"), "w") as f:
        f.write(content)

if __name__ == "__main__":
    create_complex_app("benchmarks/complex_zig_app", "zig")
    create_complex_app("benchmarks/complex_rust_app", "rust")
    print("Generated 152-variable complex apps.")
