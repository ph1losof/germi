use criterion::{black_box, criterion_group, criterion_main, Criterion};
use germi::Germi;

fn benchmark_interpolator(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpolator");
    
    // Setup generic engine
    let mut germi = Germi::default();
    germi.add_variable("KEY", "value");
    germi.add_variable("USER", "tentacles");
    germi.add_variable("HOST", "localhost");
    germi.add_variable("PORT", "8080");

    group.bench_function("simple_var", |b| {
        b.iter(|| {
            let _ = germi.interpolate(black_box("Hello ${USER}"));
        })
    });

    group.bench_function("multiple_vars", |b| {
        b.iter(|| {
            let _ = germi.interpolate(black_box("Connect to ${USER}@${HOST}:${PORT}"));
        })
    });

    // Nested/Recursive setup
    let mut recursive_germi = Germi::default();
    recursive_germi.add_variable("A", "${B}");
    recursive_germi.add_variable("B", "${C}");
    recursive_germi.add_variable("C", "final_value");

    group.bench_function("nested_vars", |b| {
        b.iter(|| {
            let _ = recursive_germi.interpolate(black_box("${A}"));
        })
    });

    // Large payload
    let mut large_germi = Germi::default();
    let mut large_payload = String::new();
    for i in 0..100 {
        large_germi.add_variable(format!("KEY_{}", i), format!("value_{}", i));
        large_payload.push_str(&format!("Key {}: ${{KEY_{}}}\n", i, i));
    }

    group.bench_function("large_payload_100_vars", |b| {
        b.iter(|| {
            let _ = large_germi.interpolate(black_box(&large_payload));
        })
    });

    // No-op (Literal)
    group.bench_function("literal_noop", |b| {
        b.iter(|| {
            let _ = germi.interpolate(black_box("Just a plain string without variables"));
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_interpolator);
criterion_main!(benches);
