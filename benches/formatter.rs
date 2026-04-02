use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

fn parse_smoke_benchmark(c: &mut Criterion) {
    let src = r#"
cmake_minimum_required(VERSION 3.28)
project(cmakefmt LANGUAGES C CXX)

add_library(cmakefmt src/lib.rs)
target_link_libraries(cmakefmt
    PUBLIC fmt::fmt
    PRIVATE internal_dep
)
"#;

    c.bench_function("parse_smoke", |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(src)).expect("benchmark parse should succeed"))
    });
}

criterion_group!(benches, parse_smoke_benchmark);
criterion_main!(benches);
