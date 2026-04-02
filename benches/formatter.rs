use std::hint::black_box;

use cmakefmt::formatter;
use cmakefmt::spec::registry::CommandRegistry;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::prelude::*;
use regex::Regex;
use tempfile::tempdir;

fn small_source() -> &'static str {
    r#"
cmake_minimum_required(VERSION 3.28)
project(cmakefmt LANGUAGES C CXX)
add_library(cmakefmt src/lib.rs)
target_link_libraries(cmakefmt PUBLIC fmt::fmt PRIVATE internal_dep)
"#
}

fn comment_heavy_source() -> String {
    let mut out = String::new();
    for index in 0..120 {
        out.push_str(&format!("# this is comment block line {index}\n"));
        out.push_str("message(STATUS \"comment-heavy benchmark\")\n");
    }
    out
}

fn barrier_heavy_source() -> String {
    let mut out = String::new();
    for index in 0..40 {
        out.push_str("set(FOO value)\n");
        out.push_str("# cmakefmt: off\n");
        out.push_str(&format!(
            "this is intentionally invalid cmake block {index}\n"
        ));
        out.push_str("# cmakefmt: on\n");
        out.push_str("set(BAR value)\n");
        out.push_str("# ~~~\n");
        out.push_str("set(   BROKEN    value )\n");
        out.push_str("# ~~~\n");
    }
    out
}

fn large_synthetic_source() -> String {
    let mut out = String::new();
    out.push_str("cmake_minimum_required(VERSION 3.28)\n");
    out.push_str("project(PerfLarge LANGUAGES C CXX)\n");
    for index in 0..320 {
        out.push_str(&format!(
            "target_sources(perf_target PRIVATE src/file_{index}.cpp include/file_{index}.hpp)\n"
        ));
        out.push_str(&format!(
            "target_compile_definitions(perf_target PRIVATE FEATURE_{index}=1)\n"
        ));
        out.push_str(&format!(
            "install(TARGETS perf_target EXPORT PerfTargets COMPONENT runtime INCLUDES DESTINATION include/perf/{index})\n"
        ));
        if index % 8 == 0 {
            out.push_str("if(ENABLE_FEATURE AND NOT DISABLE_FEATURE)\n");
            out.push_str("  message(STATUS \"branch\")\n");
            out.push_str("endif()\n");
        }
    }
    out
}

fn parse_benchmarks(c: &mut Criterion) {
    let real_world = include_str!("../tests/fixtures/real_world/qtbase_network/CMakeLists.txt");
    let large = large_synthetic_source();

    c.bench_function("parse/small", |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(small_source())).expect("parse should succeed"))
    });
    c.bench_function("parse/real_world_qtbase_network", |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(real_world)).expect("parse should succeed"))
    });
    c.bench_function("parse/large_synthetic", |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(&large)).expect("parse should succeed"))
    });
}

fn formatter_only_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let registry = CommandRegistry::builtins();
    let small = cmakefmt::parser::parse(small_source()).expect("parse should succeed");
    let real_world = cmakefmt::parser::parse(include_str!(
        "../tests/fixtures/real_world/qtbase_network/CMakeLists.txt"
    ))
    .expect("parse should succeed");
    let large = cmakefmt::parser::parse(&large_synthetic_source()).expect("parse should succeed");

    c.bench_function("format_ast/small", |b| {
        b.iter(|| formatter::format_file(black_box(&small), &config, registry).expect("format"))
    });
    c.bench_function("format_ast/real_world_qtbase_network", |b| {
        b.iter(|| {
            formatter::format_file(black_box(&real_world), &config, registry).expect("format")
        })
    });
    c.bench_function("format_ast/large_synthetic", |b| {
        b.iter(|| formatter::format_file(black_box(&large), &config, registry).expect("format"))
    });
}

fn end_to_end_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let real_world = include_str!("../tests/fixtures/real_world/qtbase_network/CMakeLists.txt");
    let comment_heavy = comment_heavy_source();
    let large = large_synthetic_source();

    c.bench_function("format_source/small", |b| {
        b.iter(|| cmakefmt::format_source(black_box(small_source()), &config).expect("format"))
    });
    c.bench_function("format_source/real_world_qtbase_network", |b| {
        b.iter(|| cmakefmt::format_source(black_box(real_world), &config).expect("format"))
    });
    c.bench_function("format_source/comment_heavy", |b| {
        b.iter(|| cmakefmt::format_source(black_box(&comment_heavy), &config).expect("format"))
    });
    c.bench_function("format_source/large_synthetic", |b| {
        b.iter(|| cmakefmt::format_source(black_box(&large), &config).expect("format"))
    });
}

fn debug_and_barrier_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let barrier_heavy = barrier_heavy_source();

    c.bench_function("format_source_with_debug/barrier_heavy", |b| {
        b.iter(|| {
            cmakefmt::format_source_with_debug(black_box(&barrier_heavy), &config).expect("format")
        })
    });
}

fn discovery_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    for index in 0..80 {
        let cmake_file = dir.path().join(format!("src/module_{index}.cmake"));
        let ignored_file = dir.path().join(format!("src/notes_{index}.txt"));
        std::fs::create_dir_all(cmake_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&cmake_file, "set(FOO bar)\n").expect("write cmake");
        std::fs::write(&ignored_file, "ignore me\n").expect("write txt");
    }
    let filter = Regex::new("module_(1|3|5|7)").expect("regex");

    c.bench_function("file_discovery/discover_cmake_files", |b| {
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

    c.bench_function("config/config_for_file", |b| {
        b.iter(|| cmakefmt::Config::for_file(black_box(&nested)).expect("config should load"))
    });
}

fn check_and_write_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let source = "set(  FOO  bar )\n";
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("CMakeLists.txt");

    c.bench_function("check_mode/smoke", |b| {
        b.iter(|| {
            let formatted =
                cmakefmt::format_source(black_box(source), &config).expect("format should succeed");
            black_box(formatted != source)
        })
    });

    c.bench_function("in_place_write/smoke", |b| {
        b.iter(|| {
            let formatted =
                cmakefmt::format_source(black_box(source), &config).expect("format should succeed");
            std::fs::write(&output_path, formatted).expect("write should succeed");
        })
    });
}

fn batch_scaling_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let registry = CommandRegistry::builtins();
    let sources: Vec<String> = (0..48)
        .map(|index| {
            format!(
                "set(VAR_{index} value)\n\
                 target_link_libraries(target_{index} PUBLIC dep_{index} PRIVATE helper_{index})\n\
                 install(TARGETS target_{index} EXPORT Export{index} COMPONENT runtime INCLUDES DESTINATION include/{index})\n"
            )
        })
        .collect();

    let mut group = c.benchmark_group("batch_format");
    for jobs in [1usize, 4usize] {
        group.bench_with_input(BenchmarkId::from_parameter(jobs), &jobs, |b, &jobs| {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(jobs)
                .build()
                .expect("pool");

            b.iter(|| {
                pool.install(|| {
                    sources
                        .par_iter()
                        .map(|source| {
                            formatter::format_source_with_registry(
                                black_box(source),
                                &config,
                                registry,
                            )
                            .expect("format")
                        })
                        .collect::<Vec<_>>()
                })
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    parse_benchmarks,
    formatter_only_benchmarks,
    end_to_end_benchmarks,
    debug_and_barrier_benchmarks,
    discovery_benchmark,
    config_benchmark,
    check_and_write_benchmarks,
    batch_scaling_benchmarks
);
criterion_main!(benches);
