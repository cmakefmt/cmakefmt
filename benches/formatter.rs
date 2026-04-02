use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use regex::Regex;
use tempfile::tempdir;

fn parse_benchmarks(c: &mut Criterion) {
    let small = r#"
cmake_minimum_required(VERSION 3.28)
project(cmakefmt LANGUAGES C CXX)
add_library(cmakefmt src/lib.rs)
target_link_libraries(cmakefmt PUBLIC fmt::fmt PRIVATE internal_dep)
"#;
    let real_world = include_str!("../tests/fixtures/real_world/qtbase_network/CMakeLists.txt");

    c.bench_function("parse_smoke", |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(small)).expect("parse should succeed"))
    });
    c.bench_function("parse_real_world_qtbase_network", |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(real_world)).expect("parse should succeed"))
    });
}

fn format_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let small = r#"
cmake_minimum_required(VERSION 3.28)
project(cmakefmt LANGUAGES C CXX)
add_library(cmakefmt src/lib.rs)
target_link_libraries(cmakefmt PUBLIC fmt::fmt PRIVATE internal_dep)
"#;
    let real_world = include_str!("../tests/fixtures/real_world/qtbase_network/CMakeLists.txt");

    c.bench_function("format_smoke", |b| {
        b.iter(|| {
            cmakefmt::format_source(black_box(small), &config).expect("format should succeed")
        })
    });
    c.bench_function("format_real_world_qtbase_network", |b| {
        b.iter(|| {
            cmakefmt::format_source(black_box(real_world), &config).expect("format should succeed")
        })
    });
}

fn discovery_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    for index in 0..40 {
        let cmake_file = dir.path().join(format!("src/module_{index}.cmake"));
        let ignored_file = dir.path().join(format!("src/notes_{index}.txt"));
        std::fs::create_dir_all(cmake_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&cmake_file, "set(FOO bar)\n").expect("write cmake");
        std::fs::write(&ignored_file, "ignore me\n").expect("write txt");
    }
    let filter = Regex::new("module").expect("regex");

    c.bench_function("discover_cmake_files", |b| {
        b.iter(|| cmakefmt::files::discover_cmake_files(black_box(dir.path()), Some(&filter)))
    });
}

fn config_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    std::fs::write(
        dir.path().join(".cmake-format.toml"),
        "[format]\nline_width = 100\n",
    )
    .expect("root config");
    let nested = dir.path().join("src/lib/CMakeLists.txt");
    std::fs::create_dir_all(nested.parent().expect("parent")).expect("mkdir");
    std::fs::write(&nested, "set(FOO bar)\n").expect("fixture");

    c.bench_function("config_for_file", |b| {
        b.iter(|| cmakefmt::Config::for_file(black_box(&nested)).expect("config should load"))
    });
}

fn check_and_write_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let source = "set(  FOO  bar )\n";
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("CMakeLists.txt");

    c.bench_function("check_mode_smoke", |b| {
        b.iter(|| {
            let formatted =
                cmakefmt::format_source(black_box(source), &config).expect("format should succeed");
            black_box(formatted != source)
        })
    });

    c.bench_function("in_place_write_smoke", |b| {
        b.iter(|| {
            let formatted =
                cmakefmt::format_source(black_box(source), &config).expect("format should succeed");
            std::fs::write(&output_path, formatted).expect("write should succeed");
        })
    });
}

criterion_group!(
    benches,
    parse_benchmarks,
    format_benchmarks,
    discovery_benchmark,
    config_benchmark,
    check_and_write_benchmarks
);
criterion_main!(benches);
