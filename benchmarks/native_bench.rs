// native_bench.rs
#[no_mangle]
pub extern "C" fn bench_sieve(limit: u32) -> u32 {
    let lim = limit as usize;
    let mut is_prime = vec![true; lim + 1];
    let mut count = 0u32;
    let mut p = 2;
    while p * p <= lim {
        if is_prime[p] {
            let mut i = p * p;
            while i <= lim {
                is_prime[i] = false;
                i += p;
            }
        }
        p += 1;
    }
    for i in 2..=lim {
        if is_prime[i] {
            count += 1;
        }
    }
    count
}

#[no_mangle]
pub extern "C" fn bench_matrix(n: u32) -> f64 {
    let size = n as usize;
    let mut a = vec![1.0f64; size * size];
    let mut b = vec![2.0f64; size * size];
    let mut c = vec![0.0f64; size * size];

    for i in 0..size {
        for k in 0..size {
            let r = a[i * size + k];
            for j in 0..size {
                c[i * size + j] += r * b[k * size + j];
            }
        }
    }

    let mut sum = 0.0f64;
    for v in c {
        sum += v;
    }
    sum
}

#[no_mangle]
pub extern "C" fn bench_fib(n: u32) -> u64 {
    if n <= 1 {
        return n as u64;
    }
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}
