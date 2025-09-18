use criterion::{Criterion, criterion_group, criterion_main};
use mdref::find_references;
use std::hint::black_box;
use std::path::Path;

mod mock_generator;

fn benchmark_find_references(c: &mut Criterion) {
    // Generate mock data before running benchmarks
    mock_generator::generate().expect("Failed to generate mock data");

    let filepath = Path::new("mock_data/root.md");
    let root = Path::new("mock_data");

    println!("Setting up benchmark");

    c.bench_function("find_references", |b| {
        b.iter(|| {
            let result = find_references(black_box(filepath), black_box(root));
            match result {
                Ok(refs) => {
                    let _ = black_box(refs);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    panic!("Benchmark failed");
                }
            }
        })
    });
}

criterion_group!(benches, benchmark_find_references);
criterion_main!(benches);
