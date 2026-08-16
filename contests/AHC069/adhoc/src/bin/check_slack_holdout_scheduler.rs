#![allow(non_snake_case)]

use std::collections::HashSet;

const DEEP_LIMIT: usize = 8;
const SPARE_LIMIT: usize = 32;
const SPARE_SAMPLES: usize = 9;
const SPARE_SEED_OFFSET: usize = 17;
const GAP_RATIO: f64 = 0.010;
const PACE_START_RATIO: f64 = 10.0 / 190.0;
const PACE_SPAN_RATIO: f64 = 170.0 / 190.0;
const PACE_END_RATIO: f64 = 180.0 / 190.0;
const MIN_SLACK_RATIO: f64 = 1.0 / 190.0;
const HARD_RATIO: f64 = 184.0 / 190.0;

#[inline(always)]
fn paced_session_limit(limit: usize, incoming_id: usize, M: usize) -> usize {
    let M = M.max(1);
    ((limit * (incoming_id + 1) + M - 1) / M).clamp(1, limit)
}

fn pace_ratio(incoming_id: usize, M: usize) -> f64 {
    let progress = (incoming_id + 1) as f64 / M.max(1) as f64;
    (PACE_START_RATIO + PACE_SPAN_RATIO * progress).min(PACE_END_RATIO)
}

fn eligible(
    incoming_id: usize,
    M: usize,
    deep_sessions: usize,
    spare_sessions: usize,
    relative_gap: f64,
    real_elapsed_ratio: f64,
) -> bool {
    let deadline = pace_ratio(incoming_id, M).min(HARD_RATIO);
    deep_sessions >= paced_session_limit(DEEP_LIMIT, incoming_id, M)
        && spare_sessions < paced_session_limit(SPARE_LIMIT, incoming_id, M)
        && relative_gap <= GAP_RATIO
        && real_elapsed_ratio + MIN_SLACK_RATIO <= deadline
}

fn seed(incoming_id: usize, sample_index: usize) -> u64 {
    incoming_id as u64 * 1_000_003 + sample_index as u64 * 7_919 + 1
}

fn choose_after_holdout(
    base_winner: usize,
    second: usize,
    completed: usize,
    best_score: f64,
    second_score: f64,
) -> usize {
    if completed != SPARE_SAMPLES {
        base_winner
    } else if second_score > best_score {
        second
    } else {
        base_winner
    }
}

fn main() {
    let mut scheduler_checks = 0_usize;
    for M in 1..=1_000 {
        let mut previous_deep = 0;
        let mut previous_spare = 0;
        for incoming_id in 0..M {
            let deep = paced_session_limit(DEEP_LIMIT, incoming_id, M);
            let spare = paced_session_limit(SPARE_LIMIT, incoming_id, M);
            let expected_deep = ((DEEP_LIMIT * (incoming_id + 1) + M - 1) / M)
                .clamp(1, DEEP_LIMIT);
            let expected_spare = ((SPARE_LIMIT * (incoming_id + 1) + M - 1) / M)
                .clamp(1, SPARE_LIMIT);
            assert_eq!(deep, expected_deep);
            assert_eq!(spare, expected_spare);
            assert!(deep >= previous_deep && spare >= previous_spare);
            assert!((PACE_START_RATIO..=PACE_END_RATIO).contains(&pace_ratio(incoming_id, M)));
            previous_deep = deep;
            previous_spare = spare;
            scheduler_checks += 1;
        }
        assert_eq!(previous_deep, DEEP_LIMIT);
        assert_eq!(previous_spare, SPARE_LIMIT);
    }

    for incoming_id in 0..1_000 {
        let M = 1_000;
        let deep = paced_session_limit(DEEP_LIMIT, incoming_id, M);
        let spare = paced_session_limit(SPARE_LIMIT, incoming_id, M);
        let deadline = pace_ratio(incoming_id, M);
        assert!(!eligible(
            incoming_id,
            M,
            deep.saturating_sub(1),
            0,
            0.0,
            0.0
        ));
        assert!(!eligible(
            incoming_id,
            M,
            deep,
            spare,
            0.0,
            0.0
        ));
        assert!(!eligible(
            incoming_id,
            M,
            deep,
            0,
            GAP_RATIO + 1e-12,
            0.0
        ));
        assert!(!eligible(
            incoming_id,
            M,
            deep,
            0,
            0.0,
            deadline - MIN_SLACK_RATIO + 1e-12
        ));
        assert!(eligible(
            incoming_id,
            M,
            deep,
            spare - 1,
            GAP_RATIO,
            deadline - MIN_SLACK_RATIO
        ));
    }

    let mut standard_and_central = HashSet::new();
    let mut spare = HashSet::new();
    for incoming_id in 0..1_000 {
        for sample_index in 0..=3 {
            assert!(standard_and_central.insert(seed(incoming_id, sample_index)));
        }
        for k in 0..SPARE_SAMPLES {
            let value = seed(incoming_id, SPARE_SEED_OFFSET + k);
            assert!(!standard_and_central.contains(&value));
            assert!(spare.insert(value));
        }
    }
    assert!(standard_and_central.is_disjoint(&spare));

    for completed in 0..SPARE_SAMPLES {
        assert_eq!(choose_after_holdout(2, 1, completed, 0.0, 1.0), 2);
    }
    assert_eq!(choose_after_holdout(2, 1, SPARE_SAMPLES, 0.0, 1.0), 1);
    assert_eq!(choose_after_holdout(2, 1, SPARE_SAMPLES, 1.0, 1.0), 2);

    println!(
        "verified scheduler_states={} standard_seeds={} spare_seeds={} incomplete_keeps_base={}",
        scheduler_checks,
        standard_and_central.len(),
        spare.len(),
        SPARE_SAMPLES
    );
}
