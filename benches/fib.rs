use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use drsm::Machine;
use std::hint::black_box;

fn fib_machine(n: i64) -> Machine {
    assert!(n < 93, "Too big for i64");
    let mut m = Machine::default();
    m.read_eval("def fib_0 1").expect("OK by design");
    m.read_eval("def fib_1 1").expect("OK by design");
    (0..n - 1).for_each(|i| {
        m.read_eval(&format!("def fib_{} fib_{} fib_{i} add", i + 2, i + 1))
            .expect("OK by design");
    });
    m
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci");
    for n in 10..20 {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let mut m = fib_machine(n);
            b.iter(|| m.read_eval(&black_box(format!("fib_{n}"))));
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
