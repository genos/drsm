use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use drsm::Machine;
use std::hint::black_box;

fn zero_sum_machine(n: i64) -> Machine {
    let mut m = Machine::default();
    let mut s = "def zero_sum".to_string();
    for _ in 0..n {
        s.push_str(" 0 0 add");
    }
    s.push_str(" add");
    m.read_eval(&s).expect("OK by design");
    m
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Zero Sum");
    for n in 10..20 {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let mut m = zero_sum_machine(n);
            b.iter(|| m.read_eval(black_box("zero_sum")));
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
