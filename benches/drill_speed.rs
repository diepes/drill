#![allow(unused)]
use std::sync::Arc;
use std::time::Instant;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use drill::{benchmark, tags};

pub fn criterion_benchmark(c: &mut Criterion) {
  let drill_benchmark_path = "example/benchmark2.yml";
  let mut bench_cnt : f64 = 0.0;
  let mut bench_total: f64 = 0.0;
  let mut bench_worst: f64 = 0.0;
  c.bench_function("drill benchmark", |b| {
    b.iter(|| {
      let start = Instant::now();
      bench_cnt += 1.0;
      let result = benchmark::execute(drill_benchmark_path , None, false, false, true, false, Some(&String::from("3")), false, &tags::Tags::new(None, None));
      let elapsed = start.elapsed().as_secs_f64();
      bench_total += elapsed;
      bench_worst = bench_worst.max(elapsed);
      println!("==> bencher #{} {:.3}s ave_{:.3}s r_{:.3}s w_{:.3}s",bench_cnt, elapsed, bench_total / bench_cnt, result.duration, bench_worst);
      println!();
    })
  });
  println!("drill benchmark {}", drill_benchmark_path);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
