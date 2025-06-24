use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use ftml_uris::prelude::*;
use std::{path::Path, str::FromStr};

fn bench_base_uri_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("base_uri_parsing");

    // Different URL complexities
    let urls = vec![
        ("simple", "http://example.com"),
        ("with_path", "http://example.com/path/to/resource"),
        ("with_port", "http://example.com:8080/path"),
        (
            "complex",
            "https://user:pass@example.com:443/path/to/resource",
        ),
    ];

    for (name, url) in urls {
        group.bench_with_input(BenchmarkId::from_parameter(name), &url, |b, url| {
            b.iter(|| BaseUri::from_str(black_box(url)).expect("impossible"));
        });
    }

    // Benchmark repeated parsing of the same URL (should hit cache)
    group.bench_function("cached", |b| {
        b.iter(|| BaseUri::from_str(black_box("http://example.com")).expect("impossible"));
    });

    // Benchmark parsing many different URLs (cache misses)
    let many_urls: Vec<String> = (0..100).map(|i| format!("http://example{i}.com")).collect();

    group.bench_function("many_unique", |b| {
        let mut i = 0;
        b.iter(|| {
            let url = &many_urls[i % many_urls.len()];
            i += 1;
            BaseUri::from_str(black_box(url)).expect("impossible")
        });
    });

    group.finish();
}

fn bench_archive_uri_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_uri_parsing");

    let archive_uris = vec![
        ("simple", "http://example.com?a=archive"),
        ("nested", "http://example.com?a=org/project/archive"),
        (
            "deep",
            "http://example.com?a=org/division/team/project/subproject/archive",
        ),
    ];

    for (name, uri) in archive_uris {
        group.bench_with_input(BenchmarkId::from_parameter(name), &uri, |b, uri| {
            b.iter(|| ArchiveUri::from_str(black_box(uri)).expect("impossible"));
        });
    }

    group.finish();
}

fn bench_path_uri_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_uri_parsing");

    let path_uris = vec![
        ("no_path", "http://example.com?a=archive"),
        ("simple_path", "http://example.com?a=archive&p=file"),
        (
            "nested_path",
            "http://example.com?a=archive&p=folder/subfolder/file",
        ),
        (
            "deep_path",
            "http://example.com?a=archive&p=a/b/c/d/e/f/g/h/i/j/file",
        ),
    ];

    for (name, uri) in path_uris {
        group.bench_with_input(BenchmarkId::from_parameter(name), &uri, |b, uri| {
            b.iter(|| PathUri::from_str(black_box(uri)).expect("impossible"));
        });
    }

    group.finish();
}

fn bench_path_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_navigation");

    let paths = vec![
        ("short", "a/b"),
        ("medium", "a/b/c/d/e"),
        ("long", "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p"),
    ];

    for (name, path_str) in paths {
        let path = UriPath::from_str(path_str).expect("impossible");

        group.bench_with_input(BenchmarkId::new("up", name), &path, |b, path| {
            b.iter(|| path.up());
        });

        // Benchmark going all the way up
        group.bench_with_input(BenchmarkId::new("up_all", name), &path, |b, path| {
            b.iter(|| {
                let mut current = Some(path.clone());
                while let Some(p) = current {
                    current = p.up();
                }
            });
        });
    }

    group.finish();
}

fn bench_language_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("language_parsing");
    let paths = [
        "file.en.tex",
        "document.de.html",
        "README.fr.md",
        "test.xx.txt", // Unknown language
        "noext",       // No extension
    ];

    group.bench_function("from_path", |b| {
        let mut i = 0;
        b.iter(|| {
            let path = Path::new(black_box(paths[i % paths.len()]));
            i += 1;
            Language::from(path)
        });
    });

    group.bench_function("from_rel_path", |b| {
        let mut i = 0;
        b.iter(|| {
            let path = paths[i % paths.len()];
            i += 1;
            Language::from_rel_path(black_box(path))
        });
    });

    group.finish();
}

// Baseline implementations for comparison
mod baseline {
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct NaiveBaseUri(url::Url);

    impl std::str::FromStr for NaiveBaseUri {
        type Err = url::ParseError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(Self(url::Url::parse(s)?))
        }
    }

    #[derive(Clone, PartialEq, Eq)]
    pub struct SimpleInternedBaseUri(std::sync::Arc<url::Url>);

    static SIMPLE_CACHE: std::sync::LazyLock<Mutex<HashMap<String, std::sync::Arc<url::Url>>>> =
        std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

    impl std::str::FromStr for SimpleInternedBaseUri {
        type Err = url::ParseError;

        #[allow(clippy::significant_drop_tightening)]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut cache = SIMPLE_CACHE.lock().expect("impossible");
            if let Some(arc) = cache.get(s) {
                Ok(Self(arc.clone()))
            } else {
                let url = url::Url::parse(s)?;
                let arc = std::sync::Arc::new(url);
                cache.insert(s.to_string(), arc.clone());
                Ok(Self(arc))
            }
        }
    }

    #[derive(Clone, PartialEq, Eq)]
    pub struct NaivePath(String);

    impl NaivePath {
        pub fn new(s: &str) -> Self {
            Self(s.to_string())
        }

        pub fn up(&self) -> Option<Self> {
            self.0
                .rsplit_once('/')
                .map(|(parent, _)| Self(parent.to_string()))
        }
    }
}

fn bench_baseline_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_comparison");

    // Compare BaseUri parsing with naive implementation
    let url = "http://example.com/path/to/resource";

    group.bench_function("ftml_base_uri", |b| {
        b.iter(|| BaseUri::from_str(black_box(url)).expect("impossible"));
    });

    group.bench_function("naive_base_uri", |b| {
        b.iter(|| baseline::NaiveBaseUri::from_str(black_box(url)).expect("impossible"));
    });

    group.bench_function("simple_interned_base_uri", |b| {
        b.iter(|| baseline::SimpleInternedBaseUri::from_str(black_box(url)).expect("impossible"));
    });

    // Compare path operations
    let path_str = "a/b/c/d/e/f/g/h";
    let ftml_path = UriPath::from_str(path_str).expect("impossible");
    let naive_path = baseline::NaivePath::new(path_str);

    group.bench_function("ftml_path_up", |b| {
        b.iter(|| ftml_path.up());
    });

    group.bench_function("naive_path_up", |b| {
        b.iter(|| naive_path.up());
    });

    // Compare equality checks
    let ftml_path2 = UriPath::from_str(path_str).expect("impossible");
    let naive_path2 = baseline::NaivePath::new(path_str);

    group.bench_function("ftml_path_eq", |b| {
        b.iter(|| black_box(&ftml_path) == black_box(&ftml_path2));
    });

    group.bench_function("naive_path_eq", |b| {
        b.iter(|| black_box(&naive_path) == black_box(&naive_path2));
    });

    group.finish();
}

fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Benchmark creating many interned strings
    let strings: Vec<String> = (0..1000).map(|i| format!("path/to/resource/{i}")).collect();

    group.bench_function("create_many_paths", |b| {
        b.iter(|| {
            let paths: Vec<UriPath> = strings
                .iter()
                .map(|s| UriPath::from_str(s).expect("impossible"))
                .collect();
            black_box(paths)
        });
    });

    // Benchmark with repeated strings (should benefit from interning)
    let repeated_strings: Vec<&str> = (0..1000)
        .map(|i| match i % 10 {
            0 => "common/path/one",
            1 => "common/path/two",
            2 => "common/path/three",
            3 => "common/path/four",
            4 => "common/path/five",
            5 => "another/common/path",
            6 => "yet/another/path",
            7 => "frequently/used/path",
            8 => "shared/resource/path",
            _ => "default/path",
        })
        .collect();

    group.bench_function("create_repeated_paths", |b| {
        b.iter(|| {
            let paths: Vec<UriPath> = repeated_strings
                .iter()
                .map(|s| UriPath::from_str(s).expect("impossible"))
                .collect();
            black_box(paths)
        });
    });

    group.finish();
}

fn bench_archive_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_operations");

    let archive_ids = vec![
        ("simple", "myarchive"),
        ("nested", "org/project/archive"),
        (
            "deep",
            "org/division/team/project/subproject/module/archive",
        ),
        ("meta", "some/path/meta-inf"),
    ];

    for (name, id_str) in &archive_ids {
        let id = ArchiveId::from_str(id_str).expect("impossible");

        group.bench_with_input(BenchmarkId::new("first_name", name), &id, |b, id| {
            b.iter(|| id.first_name());
        });

        group.bench_with_input(BenchmarkId::new("last_name", name), &id, |b, id| {
            b.iter(|| id.last_name());
        });

        group.bench_with_input(BenchmarkId::new("is_meta", name), &id, |b, id| {
            b.iter(|| id.is_meta());
        });

        group.bench_with_input(BenchmarkId::new("steps_collect", name), &id, |b, id| {
            b.iter(|| {
                let steps: Vec<&str> = id.steps().collect();
                black_box(steps)
            });
        });
    }

    group.finish();
}

fn bench_bitand_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitand_operations");

    let base = BaseUri::from_str("http://example.com").expect("impossible");
    let archive_id = ArchiveId::from_str("my/archive/id").expect("impossible");

    group.bench_function("base_and_archive_id", |b| {
        b.iter(|| black_box(base.clone()) & black_box(archive_id.clone()));
    });

    group.bench_function("base_and_str", |b| {
        b.iter(|| {
            black_box(
                black_box(base.clone()) & black_box("my/archive/id".parse().expect("impossible")),
            )
        });
    });

    group.finish();
}

fn bench_module_uri_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("module_uri_parsing");

    let module_uris = vec![
        ("simple", "http://example.com?a=archive&m=module"),
        ("nested", "http://example.com?a=archive&m=math/algebra"),
        (
            "with_path",
            "http://example.com?a=archive&p=folder&m=module",
        ),
        (
            "complex",
            "http://example.com?a=org/project/archive&p=textbooks/advanced&m=math/analysis/real",
        ),
    ];

    for (name, uri) in module_uris {
        group.bench_with_input(BenchmarkId::from_parameter(name), &uri, |b, uri| {
            b.iter(|| ModuleUri::from_str(black_box(uri)).expect("impossible"));
        });
    }

    group.finish();
}

fn bench_infix_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("infix_operations");

    let base = BaseUri::from_str("http://example.com").expect("impossible");
    let archive_id = ArchiveId::from_str("archive").expect("impossible");
    let path = UriPath::from_str("folder/file").expect("impossible");
    let name = UriName::from_str("module").expect("impossible");

    // Benchmark & operator
    group.bench_function("base_bitand_archive", |b| {
        b.iter(|| black_box(base.clone()) & black_box(archive_id.clone()));
    });

    // Benchmark / operator for paths
    let path1 = UriPath::from_str("folder").expect("impossible");
    let path2 = UriPath::from_str("file").expect("impossible");
    group.bench_function("path_div_path", |b| {
        b.iter(|| black_box(&path1) / black_box(&path2));
    });

    // Benchmark / operator for archive + path
    let archive_uri = ArchiveUri::from_str("http://example.com?a=archive").expect("impossible");
    group.bench_function("archive_div_path", |b| {
        b.iter(|| black_box(archive_uri.clone()) / black_box(path.clone()));
    });

    // Benchmark | operator
    let path_uri = PathUri::from_str("http://example.com?a=archive&p=folder").expect("impossible");
    group.bench_function("path_bitor_name", |b| {
        b.iter(|| black_box(path_uri.clone()) | black_box(name.clone()));
    });

    // Benchmark complex chaining
    #[allow(clippy::precedence)]
    group.bench_function("complex_chaining", |b| {
        b.iter(|| {
            let result = ((black_box(base.clone()) & black_box(archive_id.clone()))
                / black_box(path.clone()))
                | black_box(name.clone());
            black_box(result)
        });
    });

    group.finish();
}

fn bench_uri_name_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("uri_name_operations");

    let names = vec![
        ("simple", "module"),
        ("nested", "math/algebra"),
        ("deep", "math/analysis/real/sequences/convergence"),
    ];

    for (bench_name, name_str) in &names {
        let name = UriName::from_str(name_str).expect("impossible");

        group.bench_with_input(BenchmarkId::new("first", bench_name), &name, |b, name| {
            b.iter(|| name.first());
        });

        group.bench_with_input(BenchmarkId::new("last", bench_name), &name, |b, name| {
            b.iter(|| name.last());
        });

        group.bench_with_input(BenchmarkId::new("is_top", bench_name), &name, |b, name| {
            b.iter(|| name.is_simple());
        });

        group.bench_with_input(
            BenchmarkId::new("steps_count", bench_name),
            &name,
            |b, name| {
                b.iter(|| {
                    let count = name.steps().count();
                    black_box(count)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("steps_collect", bench_name),
            &name,
            |b, name| {
                b.iter(|| {
                    let steps: Vec<&str> = name.steps().collect();
                    black_box(steps)
                });
            },
        );

        if !name.is_simple() {
            group.bench_with_input(BenchmarkId::new("up", bench_name), &name, |b, name| {
                b.iter(|| name.clone().up());
            });

            group.bench_with_input(BenchmarkId::new("top", bench_name), &name, |b, name| {
                b.iter(|| name.clone().top());
            });
        }
    }

    group.finish();
}

fn bench_uri_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("uri_conversions");

    let module_uri =
        ModuleUri::from_str("http://example.com?a=archive&p=path&m=module").expect("impossible");

    // Benchmark trait conversions
    group.bench_function("module_to_base", |b| {
        b.iter(|| {
            let base: BaseUri = black_box(module_uri.clone()).into();
            black_box(base)
        });
    });

    group.bench_function("module_to_archive", |b| {
        b.iter(|| {
            let archive: ArchiveUri = black_box(module_uri.clone()).into();
            black_box(archive)
        });
    });

    group.bench_function("module_to_path", |b| {
        b.iter(|| {
            let path: PathUri = black_box(module_uri.clone()).into();
            black_box(path)
        });
    });

    // Benchmark trait method calls
    group.bench_function("module_base_method", |b| {
        b.iter(|| black_box(&module_uri).base());
    });

    group.bench_function("module_archive_id", |b| {
        b.iter(|| black_box(&module_uri).archive_id());
    });

    group.bench_function("module_path", |b| {
        b.iter(|| black_box(&module_uri).path());
    });

    group.bench_function("module_name", |b| {
        b.iter(|| black_box(&module_uri).module_name());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_base_uri_parsing,
    bench_archive_uri_parsing,
    bench_path_uri_parsing,
    bench_module_uri_parsing,
    bench_path_navigation,
    bench_language_parsing,
    bench_baseline_comparison,
    bench_memory_patterns,
    bench_archive_operations,
    bench_bitand_operations,
    bench_infix_operations,
    bench_uri_name_operations,
    bench_uri_conversions
);

criterion_main!(benches);
