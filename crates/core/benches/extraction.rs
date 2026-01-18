use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lectito_core::{Document, ExtractConfig, extract_content, parse, preprocess_html};

fn fixture_path(path: &str) -> String {
    let workspace_relative = format!("../../{}", path);
    if std::path::Path::new(path).exists() {
        path.to_string()
    } else if std::path::Path::new(&workspace_relative).exists() {
        workspace_relative
    } else {
        path.to_string()
    }
}

fn bench_parse(c: &mut Criterion) {
    let small = std::fs::read_to_string(fixture_path("tests/fixtures/sites/github/article.html")).unwrap();
    let medium = std::fs::read_to_string(fixture_path("tests/fixtures/sites/wikipedia/article.html")).unwrap();
    let large = std::fs::read_to_string(fixture_path("tests/fixtures/deeply_nested.html")).unwrap();

    let mut group = c.benchmark_group("parse");

    group.bench_with_input(BenchmarkId::new("small", "5KB"), &small, |b, html| {
        b.iter(|| Document::parse(black_box(html)))
    });

    group.bench_with_input(BenchmarkId::new("medium", "50KB"), &medium, |b, html| {
        b.iter(|| Document::parse(black_box(html)))
    });

    group.bench_with_input(BenchmarkId::new("large", "500KB"), &large, |b, html| {
        b.iter(|| Document::parse(black_box(html)))
    });

    group.finish();
}

fn bench_full_extraction(c: &mut Criterion) {
    let html = std::fs::read_to_string(fixture_path("tests/fixtures/sites/wikipedia/article.html")).unwrap();

    let mut group = c.benchmark_group("extraction");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(100);

    group.bench_function("full_extraction", |b| b.iter(|| parse(black_box(&html))));

    group.finish();
}

fn bench_preprocess(c: &mut Criterion) {
    let html = std::fs::read_to_string(fixture_path("tests/fixtures/sites/wikipedia/article.html")).unwrap();
    let config = Default::default();

    c.bench_function("preprocess", |b| b.iter(|| preprocess_html(black_box(&html), &config)));
}

fn bench_scoring(c: &mut Criterion) {
    let html = std::fs::read_to_string(fixture_path("tests/fixtures/sites/wikipedia/article.html")).unwrap();
    let preprocessed = preprocess_html(&html, &Default::default());
    let doc = Document::parse(&preprocessed).unwrap();
    let config = ExtractConfig::default();

    c.bench_function("scoring_and_selection", |b| {
        b.iter(|| extract_content(black_box(&doc), black_box(&config)))
    });
}

criterion_group!(
    benches,
    bench_parse,
    bench_full_extraction,
    bench_preprocess,
    bench_scoring
);
criterion_main!(benches);
