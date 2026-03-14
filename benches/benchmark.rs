use criterion::{Criterion, criterion_group, criterion_main};
use mdref::find_references;
use std::hint::black_box;

mod mock_generator;

fn benchmark_find_references(c: &mut Criterion) {
    // Generate mock data in a temporary directory before running benchmarks
    let (_temp_dir, root_path) = mock_generator::generate().expect("Failed to generate mock data");
    let filepath = root_path.join("root.md");

    println!("Setting up benchmark in temp dir: {:?}", root_path);

    c.bench_function("find_references", |b| {
        b.iter(|| {
            let result = find_references(black_box(&filepath), black_box(&root_path));
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
