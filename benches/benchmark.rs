#![allow(clippy::type_complexity)]
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use drsm::{Core, Machine, Word};
use indexmap::IndexMap;
use itertools::Itertools;
use std::hint::black_box;

fn fib_machine(n: u32) -> Machine {
    let mut env = (0..=n)
        .tuple_windows()
        .map(|(i, j, k)| {
            (
                format!("fib_{k}"),
                vec![
                    Word::Custom(format!("fib_{j}")),
                    Word::Custom(format!("fib_{i}")),
                    Word::Core(Core::Add),
                ],
            )
        })
        .collect::<IndexMap<_, _>>();
    env.insert("fib_0".to_string(), vec![Word::Num(1)]);
    env.insert("fib_1".to_string(), vec![Word::Num(1)]);
    Machine::with_env(env)
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci");
    for n in 1..30 {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let mut m = fib_machine(n);
            b.iter(|| m.read_eval(&black_box(format!("fib_{n}"))));
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
