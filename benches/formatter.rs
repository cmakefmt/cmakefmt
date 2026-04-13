// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use cmakefmt::files::{discover_cmake_files_with_options, DiscoveryOptions};
use cmakefmt::formatter;
use cmakefmt::spec::registry::CommandRegistry;
use codspeed_criterion_compat::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode,
};
use rayon::prelude::*;
use regex::Regex;
use similar::TextDiff;
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

fn representative_real_world_source() -> String {
    let root = std::env::var_os("CMAKEFMT_REAL_WORLD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/real-world-corpus"));
    let preferred = root.join("qtbase_network/CMakeLists.txt");
    std::fs::read_to_string(&preferred).unwrap_or_else(|_| {
        include_str!("../tests/fixtures/real_world/monorepo_root.cmake").to_string()
    })
}

fn parse_benchmarks(c: &mut Criterion) {
    let real_world = representative_real_world_source();
    let large = large_synthetic_source();
    let mut group = c.benchmark_group("parse");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(80);
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function(BenchmarkId::from_parameter("small"), |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(small_source())).expect("parse should succeed"))
    });
    group.bench_function(
        BenchmarkId::from_parameter("representative_real_world"),
        |b| {
            b.iter(|| {
                cmakefmt::parser::parse(black_box(&real_world)).expect("parse should succeed")
            })
        },
    );
    group.bench_function(BenchmarkId::from_parameter("large_synthetic"), |b| {
        b.iter(|| cmakefmt::parser::parse(black_box(&large)).expect("parse should succeed"))
    });
    group.finish();
}

fn formatter_only_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let registry = CommandRegistry::builtins();
    let small_src = small_source();
    let real_world_src = representative_real_world_source();
    let large_src = large_synthetic_source();
    let small = cmakefmt::parser::parse(small_src).expect("parse should succeed");
    let real_world = cmakefmt::parser::parse(&real_world_src).expect("parse should succeed");
    let large = cmakefmt::parser::parse(&large_src).expect("parse should succeed");
    let mut group = c.benchmark_group("format_ast");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(70);
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function(BenchmarkId::from_parameter("small"), |b| {
        b.iter(|| {
            formatter::format_parsed_file(small_src, black_box(&small), &config, registry)
                .expect("format")
        })
    });
    group.bench_function(
        BenchmarkId::from_parameter("representative_real_world"),
        |b| {
            b.iter(|| {
                formatter::format_parsed_file(
                    &real_world_src,
                    black_box(&real_world),
                    &config,
                    registry,
                )
                .expect("format")
            })
        },
    );
    group.bench_function(BenchmarkId::from_parameter("large_synthetic"), |b| {
        b.iter(|| {
            formatter::format_parsed_file(&large_src, black_box(&large), &config, registry)
                .expect("format")
        })
    });
    group.finish();
}

fn end_to_end_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let real_world = representative_real_world_source();
    let comment_heavy = comment_heavy_source();
    let large = large_synthetic_source();
    let mut group = c.benchmark_group("format_source");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(60);
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function(BenchmarkId::from_parameter("small"), |b| {
        b.iter(|| cmakefmt::format_source(black_box(small_source()), &config).expect("format"))
    });
    group.bench_function(
        BenchmarkId::from_parameter("representative_real_world"),
        |b| b.iter(|| cmakefmt::format_source(black_box(&real_world), &config).expect("format")),
    );
    group.bench_function(BenchmarkId::from_parameter("comment_heavy"), |b| {
        b.iter(|| cmakefmt::format_source(black_box(&comment_heavy), &config).expect("format"))
    });
    group.bench_function(BenchmarkId::from_parameter("large_synthetic"), |b| {
        b.iter(|| cmakefmt::format_source(black_box(&large), &config).expect("format"))
    });
    group.finish();
}

fn debug_and_barrier_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let barrier_heavy = barrier_heavy_source();
    let mut group = c.benchmark_group("format_source_with_debug");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(50);
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function(BenchmarkId::from_parameter("barrier_heavy"), |b| {
        b.iter(|| {
            cmakefmt::format_source_with_debug(black_box(&barrier_heavy), &config).expect("format")
        })
    });
    group.finish();
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

    let mut group = c.benchmark_group("file_discovery");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(60);

    group.bench_function(BenchmarkId::from_parameter("discover_cmake_files"), |b| {
        b.iter(|| cmakefmt::files::discover_cmake_files(black_box(dir.path()), Some(&filter)))
    });
    group.finish();
}

fn workflow_discovery_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    std::fs::write(dir.path().join(".gitignore"), "ignored_git.cmake\n").expect("gitignore");
    std::fs::write(
        dir.path().join(".cmakefmtignore"),
        "ignored_custom.cmake\nnested/skip/**\n",
    )
    .expect("cmakefmtignore");
    std::fs::write(dir.path().join("extra.ignore"), "extra_ignored.cmake\n").expect("extra");

    for index in 0..120 {
        let keep = dir.path().join(format!("nested/keep_{index}.cmake"));
        let ignored_git = dir.path().join(format!("nested/ignored_git_{index}.cmake"));
        let ignored_custom = dir
            .path()
            .join(format!("nested/ignored_custom_{index}.cmake"));
        let ignored_extra = dir
            .path()
            .join(format!("nested/extra_ignored_{index}.cmake"));
        std::fs::create_dir_all(keep.parent().expect("parent")).expect("mkdir");
        std::fs::write(&keep, "set(FOO bar)\n").expect("write keep");
        std::fs::write(&ignored_git, "set(FOO bar)\n").expect("write ignored");
        std::fs::write(&ignored_custom, "set(FOO bar)\n").expect("write ignored");
        std::fs::write(&ignored_extra, "set(FOO bar)\n").expect("write ignored");
    }

    let filter = Regex::new("keep_").expect("regex");
    let explicit_ignore = vec![dir.path().join("extra.ignore")];
    let mut group = c.benchmark_group("workflow_discovery");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(50);

    group.bench_function(BenchmarkId::from_parameter("ignore_and_gitaware"), |b| {
        b.iter(|| {
            discover_cmake_files_with_options(
                black_box(dir.path()),
                DiscoveryOptions {
                    file_filter: Some(&filter),
                    honor_gitignore: true,
                    explicit_ignore_paths: &explicit_ignore,
                },
            )
        })
    });
    group.finish();
}

fn config_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    std::fs::write(
        dir.path().join(".cmakefmt.toml"),
        "[format]\nline_width = 100\n",
    )
    .expect("root config");
    let nested = dir.path().join("src/lib/CMakeLists.txt");
    std::fs::create_dir_all(nested.parent().expect("parent")).expect("mkdir");
    std::fs::write(&nested, "set(FOO bar)\n").expect("fixture");

    let mut group = c.benchmark_group("config");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(80);

    group.bench_function(BenchmarkId::from_parameter("config_for_file"), |b| {
        b.iter(|| cmakefmt::Config::for_file(black_box(&nested)).expect("config should load"))
    });
    group.finish();
}

fn workflow_diff_benchmark(c: &mut Criterion) {
    let original = large_synthetic_source();
    let formatted =
        cmakefmt::format_source(&original, &cmakefmt::Config::default()).expect("format");

    let mut group = c.benchmark_group("workflow_diff");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(50);

    group.bench_function(BenchmarkId::from_parameter("unified_diff_large"), |b| {
        b.iter(|| {
            TextDiff::from_lines(black_box(&original), black_box(&formatted))
                .unified_diff()
                .context_radius(3)
                .header("a/CMakeLists.txt", "b/CMakeLists.txt")
                .to_string()
        })
    });
    group.finish();
}

fn check_and_write_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config::default();
    let source = "set(  FOO  bar )\n";
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("CMakeLists.txt");

    let mut group = c.benchmark_group("io_paths");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(60);

    group.bench_function(BenchmarkId::from_parameter("check_mode_smoke"), |b| {
        b.iter(|| {
            let formatted =
                cmakefmt::format_source(black_box(source), &config).expect("format should succeed");
            black_box(formatted != source)
        })
    });

    group.bench_function(BenchmarkId::from_parameter("in_place_write_smoke"), |b| {
        b.iter(|| {
            let formatted =
                cmakefmt::format_source(black_box(source), &config).expect("format should succeed");
            std::fs::write(&output_path, formatted).expect("write should succeed");
        })
    });
    group.finish();
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
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(40);
    group.sampling_mode(SamplingMode::Flat);
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

fn config_pattern_benchmarks(c: &mut Criterion) {
    let config = cmakefmt::Config {
        literal_comment_pattern: r"^#\s*NOLINT".to_string(),
        explicit_trailing_pattern: "#<".to_string(),
        fence_pattern: r"^\s*[`~]{3}[^`\n]*$".to_string(),
        ruler_pattern: r"^[^\w\s]{3}.*[^\w\s]{3}$".to_string(),
        ..cmakefmt::Config::default()
    };

    let mut group = c.benchmark_group("config_patterns");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);

    group.bench_function(BenchmarkId::from_parameter("validate_patterns"), |b| {
        b.iter(|| black_box(&config).validate_patterns().expect("valid"))
    });
    group.finish();
}

fn legacy_conversion_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    let legacy_path = dir.path().join(".cmake-format.yaml");
    std::fs::write(
        &legacy_path,
        "format:\n  line_width: 100\n  tab_size: 4\n  use_tabchars: false\n  \
         max_lines_hwrap: 3\n  dangle_parens: true\n\
         markup:\n  enable_markup: true\n",
    )
    .expect("write legacy config");

    let mut group = c.benchmark_group("legacy_conversion");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(80);

    group.bench_function(BenchmarkId::from_parameter("convert_yaml"), |b| {
        b.iter(|| {
            cmakefmt::convert_legacy_config_files(
                black_box(std::slice::from_ref(&legacy_path)),
                cmakefmt::DumpConfigFormat::Yaml,
            )
            .expect("convert")
        })
    });
    group.finish();
}

fn atomic_write_benchmark(c: &mut Criterion) {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("CMakeLists.txt");
    let content = "set(FOO bar)\n".repeat(100);

    let mut group = c.benchmark_group("io_atomic");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(80);

    group.bench_function(BenchmarkId::from_parameter("direct_write"), |b| {
        b.iter(|| {
            std::fs::write(black_box(&target), black_box(&content)).expect("write");
        })
    });

    group.bench_function(BenchmarkId::from_parameter("atomic_write"), |b| {
        b.iter(|| {
            let mut tmp = tempfile::NamedTempFile::new_in(black_box(dir.path())).expect("tempfile");
            std::io::Write::write_all(&mut tmp, black_box(content.as_bytes())).expect("write");
            tmp.persist(black_box(&target)).expect("persist");
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    parse_benchmarks,
    formatter_only_benchmarks,
    end_to_end_benchmarks,
    debug_and_barrier_benchmarks,
    discovery_benchmark,
    workflow_discovery_benchmark,
    config_benchmark,
    workflow_diff_benchmark,
    check_and_write_benchmarks,
    batch_scaling_benchmarks,
    config_pattern_benchmarks,
    legacy_conversion_benchmark,
    atomic_write_benchmark
);
criterion_main!(benches);
