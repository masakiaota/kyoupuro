// bench_canonical_rollout.rs
#![allow(dead_code, non_snake_case)]

#[path = "../../../src/bin/v065_no_move_canonical_rollout.rs"]
mod solver;

fn main() {
    solver::verify_canonical_rollout_shapes();
    let bench = solver::benchmark_canonical_rollout(
        "tools/in/0000.txt",
        "results/out/v047_no_move_strong_biased/0000.txt",
        40,
    );
    let agreement = (bench.accept_agreements as f64) / (bench.queries as f64);
    println!("canonical shape invariants: ok");
    println!(
        "queries={} repeats={} reference_accepts={} canonical_accepts={} agreement={:.3}% both={} reference_only={} canonical_only={} same_compactness={}",
        bench.queries,
        bench.repeats,
        bench.reference_accepts,
        bench.canonical_accepts,
        100.0 * agreement,
        bench.both_accepts,
        bench.reference_only,
        bench.canonical_only,
        bench.both_accept_same_compactness,
    );
    println!(
        "reference_ms={:.3} canonical_ms={:.3} speed_ratio={:.3} recommended_samples={}",
        bench.reference_ms, bench.canonical_ms, bench.speed_ratio, bench.recommended_samples,
    );
}
