use criterion::Criterion;
use criterion::*;

use webbundle::{Bundle, Version};

/// Benchmarks for fs/builder.rs.
///
/// You have to prepare benches/bundle-{large,small} directories
/// beforehand.
async fn fs_build_large() -> Bundle {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("benches/bundle-large");

    Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir(path)
        .await
        .unwrap()
        .build()
        .unwrap()
}

fn fs_build_large_sync() -> Bundle {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("benches/bundle-large");

    Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir_sync(path)
        .unwrap()
        .build()
        .unwrap()
}

async fn fs_build_small() -> Bundle {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("benches/bundle-small");

    Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir(path)
        .await
        .unwrap()
        .build()
        .unwrap()
}

fn fs_build_large_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("fib-build-large", |b| b.to_async(&rt).iter(fs_build_large));
}

fn fs_build_large_sync_benchmark(c: &mut Criterion) {
    c.bench_function("fib-build-large-sync", |b| b.iter(fs_build_large_sync));
}

fn fs_build_small_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("fib-build-small", |b| b.to_async(&rt).iter(fs_build_small));
}

criterion_group!(
    benches,
    fs_build_large_benchmark,
    fs_build_large_sync_benchmark,
    fs_build_small_benchmark
);
criterion_main!(benches);
