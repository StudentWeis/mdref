use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mdref::{find_links, find_references};
use std::hint::black_box;

mod support;

use support::{FixtureProfile, build_fixture, run_move_operation};

const FIND_PROFILES: &[FixtureProfile] = &[
    FixtureProfile::Small,
    FixtureProfile::Medium,
    FixtureProfile::Large,
];
const MOVE_PROFILES: &[FixtureProfile] = &[FixtureProfile::Small, FixtureProfile::Medium];

fn benchmark_find_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("find");
    group.sample_size(40);

    for &profile in FIND_PROFILES {
        let fixture = build_fixture(profile).expect("benchmark fixture generation should succeed");
        let profile_label = profile.label();

        group.throughput(Throughput::Bytes(
            fixture.summary.representative_document_bytes as u64,
        ));
        group.bench_with_input(
            BenchmarkId::new("find_links", profile_label),
            &fixture.representative_document,
            |b, document| {
                b.iter(|| {
                    let result = find_links(black_box(document))
                        .expect("find_links benchmark should succeed");
                    black_box(result);
                });
            },
        );

        group.throughput(Throughput::Elements(fixture.summary.markdown_files as u64));
        group.bench_with_input(
            BenchmarkId::new("find_references_file", profile_label),
            &fixture,
            |b, fixture| {
                b.iter(|| {
                    let result =
                        find_references(black_box(&fixture.hot_file), black_box(&fixture.root))
                            .expect("find_references benchmark should succeed");
                    black_box(result);
                });
            },
        );

        group.throughput(Throughput::Elements(fixture.summary.markdown_files as u64));
        group.bench_with_input(
            BenchmarkId::new("find_references_directory", profile_label),
            &fixture,
            |b, fixture| {
                b.iter(|| {
                    let result = find_references(
                        black_box(&fixture.hot_directory),
                        black_box(&fixture.root),
                    )
                    .expect("find_references directory benchmark should succeed");
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_move_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("move");
    group.sample_size(10);

    for &profile in MOVE_PROFILES {
        let profile_label = profile.label();

        let file_fixture =
            build_fixture(profile).expect("benchmark fixture generation should succeed");
        group.throughput(Throughput::Elements(
            file_fixture.summary.hot_file_references as u64,
        ));
        group.bench_function(BenchmarkId::new("mv_file", profile_label), move |b| {
            // `iter_batched_ref` keeps fixture teardown outside the measured section.
            b.iter_batched_ref(
                || build_fixture(profile).expect("benchmark fixture generation should succeed"),
                |fixture| {
                    run_move_operation(fixture.file_move_operation())
                        .expect("mv file benchmark should succeed");
                },
                BatchSize::LargeInput,
            );
        });

        let directory_fixture =
            build_fixture(profile).expect("benchmark fixture generation should succeed");
        group.throughput(Throughput::Elements(
            directory_fixture.summary.directory_move_rewrites as u64,
        ));
        group.bench_function(BenchmarkId::new("mv_directory", profile_label), move |b| {
            // `iter_batched_ref` keeps fixture teardown outside the measured section.
            b.iter_batched_ref(
                || build_fixture(profile).expect("benchmark fixture generation should succeed"),
                |fixture| {
                    run_move_operation(fixture.directory_move_operation())
                        .expect("mv directory benchmark should succeed");
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_find_operations,
    benchmark_move_operations
);
criterion_main!(benches);
