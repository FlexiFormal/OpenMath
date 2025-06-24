use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use ftml_uris::prelude::*;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::{Arc, Barrier};
use std::thread;

fn bench_string_interning_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning_sizes");

    // Test different string sizes relative to INLINE_LEN (12 bytes)
    let test_cases = vec![
        ("tiny", "a"),
        ("small", "hello"),
        ("inline_limit", "aaaaaaaaaaaa"),      // 12
        ("just_over_inline", "aaaaaaaaaaaaa"), // 13
        (
            "medium",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ), // 50
        (
            "large",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ), // 200
        (
            "very_large",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ), // 1000
    ];

    for (name, string) in test_cases {
        let string = string.to_string();

        // First creation (cold)
        group.bench_with_input(BenchmarkId::new("first_create", name), &string, |b, s| {
            b.iter(|| {
                let path = UriPath::from_str(black_box(s)).expect("impossible");
                black_box(path);
            });
        });

        // Repeated creation (should hit interning cache)
        let _warm_up = UriPath::from_str(&string).expect("impossible");
        group.bench_with_input(
            BenchmarkId::new("interned_create", name),
            &string,
            |b, s| {
                b.iter(|| {
                    let path = UriPath::from_str(black_box(s)).expect("impossible");
                    black_box(path);
                });
            },
        );
    }

    group.finish();
}

fn bench_interning_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("interning_patterns");

    // Sequential pattern (cache friendly)
    let sequential_strings: Vec<String> =
        (0..100).map(|i| format!("path/to/resource/{i}")).collect();

    group.bench_function("sequential_unique", |b| {
        let mut i = 0;
        b.iter(|| {
            let s = &sequential_strings[i % sequential_strings.len()];
            i += 1;
            let path = UriPath::from_str(black_box(s)).expect("impossible");
            black_box(path);
        });
    });

    // Repeated pattern (high cache hit rate)
    let repeated_strings: Vec<&str> = (0..100)
        .map(|i| match i % 5 {
            0 => "common/path/one",
            1 => "common/path/two",
            2 => "common/path/three",
            3 => "common/path/four",
            _ => "common/path/five",
        })
        .collect();

    group.bench_function("repeated_pattern", |b| {
        let mut i = 0;
        b.iter(|| {
            let s = repeated_strings[i % repeated_strings.len()];
            i += 1;
            let path = UriPath::from_str(black_box(s)).expect("impossible");
            black_box(path);
        });
    });

    // Random access pattern (cache unfriendly)
    let random_indices: Vec<usize> = (0..100)
        .map(|i| (i * 37 + 11) % sequential_strings.len())
        .collect();

    group.bench_function("random_access", |b| {
        let mut i = 0;
        b.iter(|| {
            let idx = random_indices[i % random_indices.len()];
            let s = &sequential_strings[idx];
            i += 1;
            let path = UriPath::from_str(black_box(s)).expect("impossible");
            black_box(path);
        });
    });

    group.finish();
}

fn bench_pointer_equality(c: &mut Criterion) {
    let mut group = c.benchmark_group("pointer_equality");

    // Create interned strings
    let path1 = UriPath::from_str("shared/common/path").expect("impossible");
    let path2 = UriPath::from_str("shared/common/path").expect("impossible");
    let path3 = UriPath::from_str("different/path/here").expect("impossible");

    // Benchmark pointer equality (should be fast for interned strings)
    group.bench_function("equal_interned", |b| {
        b.iter(|| {
            let eq = black_box(&path1) == black_box(&path2);
            black_box(eq);
        });
    });

    group.bench_function("unequal_interned", |b| {
        b.iter(|| {
            let eq = black_box(&path1) == black_box(&path3);
            black_box(eq);
        });
    });

    // Compare with string equality
    let str1 = "shared/common/path";
    let str2 = "shared/common/path";
    let str3 = "different/path/here";

    group.bench_function("string_equal", |b| {
        b.iter(|| {
            let eq = black_box(str1) == black_box(str2);
            black_box(eq);
        });
    });

    group.bench_function("string_unequal", |b| {
        b.iter(|| {
            let eq = black_box(str1) == black_box(str3);
            black_box(eq);
        });
    });

    group.finish();
}

fn bench_concurrent_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_interning");

    for num_threads in [2, 4, 8, 16] {
        group.bench_function(format!("threads_{num_threads}"), |b| {
            b.iter(|| {
                let barrier = Arc::new(Barrier::new(num_threads));
                let handles: Vec<_> = (0..num_threads)
                    .map(|_| {
                        let barrier = Arc::clone(&barrier);
                        thread::spawn(move || {
                            barrier.wait();

                            // Each thread creates the same set of strings
                            for j in 0..10 {
                                let path = UriPath::from_str(&format!("thread/shared/path/{j}"))
                                    .expect("impossible");
                                black_box(path);
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.join().expect("impossible");
                }
            });
        });
    }

    group.finish();
}

fn bench_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure");

    // Benchmark behavior when approaching the intern limit
    group.bench_function("near_limit_performance", |b| {
        // Pre-fill to near the limit (PathStore::LIMIT = 1024)
        let prefill_count = 900;
        let _prefill: Vec<UriPath> = (0..prefill_count)
            .map(|i| UriPath::from_str(&format!("prefill/path/{i}")).expect("impossible"))
            .collect();

        let mut i = 0;
        b.iter(|| {
            let path = UriPath::from_str(&format!("bench/path/{i}")).expect("impossible");
            i = (i + 1) % 100; // Cycle through 100 different strings
            black_box(path);
        });
    });

    // Benchmark cleanup behavior
    group.bench_function("cleanup_trigger", |b| {
        b.iter(|| {
            // Create many strings to trigger cleanup
            let paths: Vec<UriPath> = (0..1500)
                .map(|i| UriPath::from_str(&format!("cleanup/test/{i}")).expect("impossible"))
                .collect();
            black_box(paths);
        });
    });

    group.finish();
}

fn bench_archive_id_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_id_interning");

    // ArchiveId has a smaller limit (128) than UriPath (1024)
    let archive_ids = vec![
        ("simple", "myarchive"),
        ("nested", "org/project/archive"),
        ("deep", "org/division/team/project/subproject/archive"),
        ("unicode", "组织/项目/存档"),
    ];

    for (name, id_str) in archive_ids {
        group.bench_with_input(BenchmarkId::new("create", name), &id_str, |b, s| {
            b.iter(|| {
                let id = ArchiveId::from_str(black_box(s)).expect("impossible");
                black_box(id);
            });
        });

        // Pre-create for equality testing
        let id1 = ArchiveId::from_str(id_str).expect("impossible");
        let id2 = ArchiveId::from_str(id_str).expect("impossible");

        group.bench_with_input(
            BenchmarkId::new("equality", name),
            &(id1, id2),
            |b, (id1, id2)| {
                b.iter(|| {
                    let eq = black_box(id1) == black_box(id2);
                    black_box(eq);
                });
            },
        );
    }

    group.finish();
}

fn bench_inline_vs_heap(c: &mut Criterion) {
    let mut group = c.benchmark_group("inline_vs_heap");

    // Strings that fit in inline storage (≤12 bytes)
    let inline_strings = [
        "a",
        "ab",
        "abc",
        "abcd",
        "abcde",
        "abcdef",
        "abcdefg",
        "abcdefgh",
        "abcdefghi",
        "abcdefghij",
        "abcdefghijk",
        "abcdefghijkl", // 12 bytes - maximum inline
    ];

    // Strings that require heap allocation
    let heap_strings = [
        "abcdefghijklm",                                      // 13 bytes - just over inline
        "aaaaaaaaaaaaaaaaaaaa",                               // 20
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // 50
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // 100
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // 200
    ];

    // Benchmark inline string operations
    group.bench_function("inline_creation_batch", |b| {
        b.iter(|| {
            let paths: Vec<UriPath> = inline_strings
                .iter()
                .map(|s| UriPath::from_str(black_box(s)).expect("impossible"))
                .collect();
            black_box(paths);
        });
    });

    // Benchmark heap string operations
    let heap_owned: Vec<String> = heap_strings.iter().map(|s| (*s).to_string()).collect();
    group.bench_function("heap_creation_batch", |b| {
        b.iter(|| {
            let paths: Vec<UriPath> = heap_owned
                .iter()
                .map(|s| UriPath::from_str(black_box(s.as_str())).expect("impossible"))
                .collect();
            black_box(paths);
        });
    });

    // Compare lookup performance
    let inline_path = UriPath::from_str("short").expect("impossible");
    let heap_path = UriPath::from_str(&"a".repeat(50)).expect("impossible");

    group.bench_function("inline_lookup", |b| {
        b.iter(|| {
            let path = UriPath::from_str(black_box("short")).expect("impossible");
            let eq = path == inline_path;
            black_box(eq);
        });
    });

    group.bench_function("heap_lookup", |b| {
        let long_str = "a".repeat(50);
        b.iter(|| {
            let path = UriPath::from_str(black_box(&long_str)).expect("impossible");
            let eq = path == heap_path;
            black_box(eq);
        });
    });

    group.finish();
}

fn bench_segment_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("segment_operations");

    let test_paths: Vec<(&str, Cow<str>)> = vec![
        ("short", "a/b".into()),
        ("medium", "a/b/c/d/e".into()),
        ("long", "a/b/c/d/e/f/g/h/i/j".into()),
        (
            "very_long",
            (0..20)
                .map(|i| format!("segment{i}"))
                .collect::<Vec<_>>()
                .join("/")
                .into(),
        ),
    ];

    for (name, path_str) in test_paths {
        let path = ArchiveId::from_str(&path_str).expect("impossible");

        group.bench_with_input(BenchmarkId::new("first_name", name), &path, |b, path| {
            b.iter(|| {
                let first = path.first_name();
                black_box(first);
            });
        });

        group.bench_with_input(BenchmarkId::new("last_name", name), &path, |b, path| {
            b.iter(|| {
                let last = path.last_name();
                black_box(last);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("iterate_segments", name),
            &path,
            |b, path| {
                b.iter(|| {
                    let count = path.steps().count();
                    black_box(count);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("collect_segments", name),
            &path,
            |b, path| {
                b.iter(|| {
                    let segments: Vec<&str> = path.steps().collect();
                    black_box(segments);
                });
            },
        );
    }

    group.finish();
}

fn bench_uri_name_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("uri_name_interning");

    // UriName has a limit of 1024 like UriPath
    let uri_names = vec![
        ("simple", "module"),
        ("nested", "math/algebra"),
        ("deep", "math/analysis/real/sequences"),
        ("unicode", "数学/代数/群论"),
    ];

    for (name, name_str) in uri_names {
        group.bench_with_input(BenchmarkId::new("create", name), &name_str, |b, s| {
            b.iter(|| {
                let name = UriName::from_str(black_box(s)).expect("impossible");
                black_box(name);
            });
        });

        // Pre-create for equality testing
        let name1 = UriName::from_str(name_str).expect("impossible");
        let name2 = UriName::from_str(name_str).expect("impossible");

        group.bench_with_input(
            BenchmarkId::new("equality", name),
            &(name1, name2),
            |b, (name1, name2)| {
                b.iter(|| {
                    let eq = black_box(name1) == black_box(name2);
                    black_box(eq);
                });
            },
        );
    }

    group.finish();
}

fn bench_name_operations_vs_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("name_operations_vs_strings");

    // Compare UriName operations with equivalent string operations
    let name_str = "math/algebra/groups/theory";
    let uri_name = UriName::from_str(name_str).expect("impossible");

    // First/last operations
    group.bench_function("uri_name_first", |b| {
        b.iter(|| {
            let first = uri_name.first();
            black_box(first);
        });
    });

    group.bench_function("string_first", |b| {
        b.iter(|| {
            let first = name_str.split('/').next().expect("impossible");
            black_box(first);
        });
    });

    group.bench_function("uri_name_last", |b| {
        b.iter(|| {
            let last = uri_name.last();
            black_box(last);
        });
    });

    group.bench_function("string_last", |b| {
        b.iter(|| {
            let last = name_str.split('/').next_back().expect("impossible");
            black_box(last);
        });
    });

    // Steps iteration
    group.bench_function("uri_name_steps", |b| {
        b.iter(|| {
            let steps: Vec<&str> = uri_name.steps().collect();
            black_box(steps);
        });
    });

    group.bench_function("string_split", |b| {
        b.iter(|| {
            let steps: Vec<&str> = name_str.split('/').collect();
            black_box(steps);
        });
    });

    // Up navigation
    group.bench_function("uri_name_up", |b| {
        b.iter(|| {
            let up = uri_name.clone().up();
            black_box(up);
        });
    });

    group.bench_function("string_up", |b| {
        b.iter(|| {
            let up = name_str.rsplit_once('/').map(|(parent, _)| parent);
            black_box(up);
        });
    });

    group.finish();
}

fn bench_interning_store_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("interning_store_limits");

    // Test behavior at different store limits
    // ArchiveId: 128, UriName: 1024, UriPath: 1024

    // Archive ID limit testing
    group.bench_function("archive_id_near_limit", |b| {
        // Pre-fill to near the limit
        let _prefill: Vec<ArchiveId> = (0..100)
            .map(|i| ArchiveId::from_str(&format!("archive{i}")).expect("impossible"))
            .collect();

        let mut i = 0;
        b.iter(|| {
            let archive = ArchiveId::from_str(&format!("test/archive/{i}")).expect("impossible");
            i = (i + 1) % 50;
            black_box(archive);
        });
    });

    // UriName limit testing
    group.bench_function("uri_name_near_limit", |b| {
        // Pre-fill to near the limit
        let _prefill: Vec<UriName> = (0..800)
            .map(|i| UriName::from_str(&format!("module{i}")).expect("impossible"))
            .collect();

        let mut i = 0;
        b.iter(|| {
            let name = UriName::from_str(&format!("test/module/{i}")).expect("impossible");
            i = (i + 1) % 100;
            black_box(name);
        });
    });

    group.finish();
}

fn bench_mixed_interning_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_interning_workload");

    // Simulate a realistic workload with mixed URI component creation
    let archives = ["math", "physics", "cs", "biology"];
    let paths = ["textbooks", "papers", "exercises", "solutions"];
    let modules = ["algebra", "analysis", "geometry", "statistics"];

    group.bench_function("mixed_creation", |b| {
        let mut counter = 0;
        b.iter(|| {
            let archive_idx = counter % archives.len();
            let path_idx = (counter / archives.len()) % paths.len();
            let module_idx = (counter / (archives.len() * paths.len())) % modules.len();

            let archive = ArchiveId::from_str(&format!("{}/archive", archives[archive_idx]))
                .expect("impossible");
            let path =
                UriPath::from_str(&format!("{}/folder", paths[path_idx])).expect("impossible");
            let module =
                UriName::from_str(&format!("{}/module", modules[module_idx])).expect("impossible");

            counter += 1;

            black_box((archive, path, module));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_string_interning_sizes,
    bench_interning_patterns,
    bench_pointer_equality,
    bench_concurrent_interning,
    bench_memory_pressure,
    bench_archive_id_interning,
    bench_inline_vs_heap,
    bench_segment_operations,
    bench_uri_name_interning,
    bench_name_operations_vs_strings,
    bench_interning_store_limits,
    bench_mixed_interning_workload
);

criterion_main!(benches);
