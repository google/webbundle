use criterion::Criterion;
use criterion::*;

use webbundle::{Bundle, Version};

/// Benchmarks for fs/builder.rs.
///
/// You have to prepare benches/bundle directories
/// beforehand.
async fn fs_build_async() -> Bundle {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("benches/bundle");

    Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir(path)
        .await
        .unwrap()
        .build()
        .unwrap()
}

fn fs_build_sync() -> Bundle {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("benches/bundle");

    Bundle::builder()
        .version(Version::VersionB2)
        .exchanges_from_dir_sync(path)
        .unwrap()
        .build()
        .unwrap()
}

fn fs_build_async_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("fs-build-async", |b| b.to_async(&rt).iter(fs_build_async));
}

fn fs_build_sync_benchmark(c: &mut Criterion) {
    c.bench_function("fs-build-sync", |b| b.iter(fs_build_sync));
}

criterion_group!(benches, fs_build_async_benchmark, fs_build_sync_benchmark,);
criterion_main!(benches);
