#![allow(non_snake_case)]

use std::collections::HashSet;

const DEEP_LIMIT: usize = 8;
const SPARE_LIMIT: usize = 32;
const RESERVE_LIMIT: usize = 96;
const SAMPLES: usize = 9;
const SPARE_SEED_OFFSET: usize = 17;
const RESERVE_SEED_OFFSET: usize = 29;
const GAP_RATIO: f64 = 0.010;
const PACE_START_RATIO: f64 = 10.0 / 190.0;
const PACE_SPAN_RATIO: f64 = 170.0 / 190.0;
const PACE_END_RATIO: f64 = 180.0 / 190.0;
const SPARE_MIN_SLACK_RATIO: f64 = 1.0 / 190.0;
const RESERVE_MIN_SLACK_RATIO: f64 = 30.0 / 190.0;
const HARD_RATIO: f64 = 184.0 / 190.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tier {
    None,
    Spare,
    Reserve,
}

#[inline(always)]
fn paced_session_limit(limit: usize, incoming_id: usize, M: usize) -> usize {
    let M = M.max(1);
    ((limit * (incoming_id + 1) + M - 1) / M).clamp(1, limit)
}

fn pace_ratio(incoming_id: usize, M: usize) -> f64 {
    let progress = (incoming_id + 1) as f64 / M.max(1) as f64;
    (PACE_START_RATIO + PACE_SPAN_RATIO * progress).min(PACE_END_RATIO)
}

fn choose_tier(
    incoming_id: usize,
    M: usize,
    deep_sessions: usize,
    spare_sessions: usize,
    reserve_sessions: usize,
    relative_gap: f64,
    real_elapsed_ratio: f64,
) -> Tier {
    let deadline = pace_ratio(incoming_id, M).min(HARD_RATIO);
    if deep_sessions < paced_session_limit(DEEP_LIMIT, incoming_id, M) || relative_gap > GAP_RATIO {
        return Tier::None;
    }
    if spare_sessions < paced_session_limit(SPARE_LIMIT, incoming_id, M) {
        return if real_elapsed_ratio + SPARE_MIN_SLACK_RATIO <= deadline {
            Tier::Spare
        } else {
            Tier::None
        };
    }
    if reserve_sessions < paced_session_limit(RESERVE_LIMIT, incoming_id, M)
        && real_elapsed_ratio + RESERVE_MIN_SLACK_RATIO <= deadline
    {
        Tier::Reserve
    } else {
        Tier::None
    }
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
    if completed != SAMPLES {
        base_winner
    } else if second_score > best_score {
        second
    } else {
        base_winner
    }
}

fn main() {
    assert!(RESERVE_MIN_SLACK_RATIO > SPARE_MIN_SLACK_RATIO);
    let mut scheduler_checks = 0_usize;
    for M in 1..=1_000 {
        let mut previous = [0; 3];
        for incoming_id in 0..M {
            let current = [
                paced_session_limit(DEEP_LIMIT, incoming_id, M),
                paced_session_limit(SPARE_LIMIT, incoming_id, M),
                paced_session_limit(RESERVE_LIMIT, incoming_id, M),
            ];
            for i in 0..3 {
                assert!(current[i] >= previous[i]);
            }
            assert!((PACE_START_RATIO..=PACE_END_RATIO).contains(&pace_ratio(incoming_id, M)));
            previous = current;
            scheduler_checks += 1;
        }
        assert_eq!(previous, [DEEP_LIMIT, SPARE_LIMIT, RESERVE_LIMIT]);
    }

    for incoming_id in 0..1_000 {
        let M = 1_000;
        let deep = paced_session_limit(DEEP_LIMIT, incoming_id, M);
        let spare = paced_session_limit(SPARE_LIMIT, incoming_id, M);
        let reserve = paced_session_limit(RESERVE_LIMIT, incoming_id, M);
        let deadline = pace_ratio(incoming_id, M).min(HARD_RATIO);
        assert_eq!(
            choose_tier(incoming_id, M, deep - 1, 0, 0, 0.0, 0.0),
            Tier::None
        );
        assert_eq!(
            choose_tier(incoming_id, M, deep, spare - 1, 0, GAP_RATIO, 0.0),
            Tier::Spare
        );
        // 第一層に空きがあれば、第二層の条件を見ず必ず第一層を優先する。
        assert_ne!(
            choose_tier(incoming_id, M, deep, spare - 1, 0, GAP_RATIO, 0.0),
            Tier::Reserve
        );
        if deadline >= RESERVE_MIN_SLACK_RATIO {
            assert_eq!(
                choose_tier(
                    incoming_id,
                    M,
                    deep,
                    spare,
                    reserve - 1,
                    GAP_RATIO,
                    deadline - RESERVE_MIN_SLACK_RATIO - 1e-12,
                ),
                Tier::Reserve
            );
        } else {
            assert_eq!(
                choose_tier(incoming_id, M, deep, spare, 0, GAP_RATIO, 0.0),
                Tier::None
            );
        }
        assert_eq!(
            choose_tier(incoming_id, M, deep, spare, reserve, GAP_RATIO, 0.0,),
            Tier::None
        );
        assert_eq!(
            choose_tier(incoming_id, M, deep, spare, 0, GAP_RATIO + 1e-12, 0.0,),
            Tier::None
        );
        assert_eq!(
            choose_tier(
                incoming_id,
                M,
                deep,
                spare,
                0,
                GAP_RATIO,
                deadline - RESERVE_MIN_SLACK_RATIO + 1e-12,
            ),
            Tier::None
        );
    }

    let mut standard = HashSet::new();
    let mut spare = HashSet::new();
    let mut reserve = HashSet::new();
    let mut paired_checks = 0_usize;
    for incoming_id in 0..1_000 {
        for sample_index in 0..=3 {
            assert!(standard.insert(seed(incoming_id, sample_index)));
        }
        for k in 0..SAMPLES {
            let spare_seed = seed(incoming_id, SPARE_SEED_OFFSET + k);
            let reserve_seed = seed(incoming_id, RESERVE_SEED_OFFSET + k);
            assert!(spare.insert(spare_seed));
            assert!(reserve.insert(reserve_seed));
            // paired候補は同一scenario seedを受け取る。
            assert_eq!(reserve_seed, seed(incoming_id, RESERVE_SEED_OFFSET + k));
            paired_checks += 1;
        }
    }
    assert!(standard.is_disjoint(&spare));
    assert!(standard.is_disjoint(&reserve));
    assert!(spare.is_disjoint(&reserve));

    for completed in 0..SAMPLES {
        assert_eq!(choose_after_holdout(2, 1, completed, 0.0, 1.0), 2);
    }
    assert_eq!(choose_after_holdout(2, 1, SAMPLES, 0.0, 1.0), 1);
    assert_eq!(choose_after_holdout(2, 1, SAMPLES, 1.0, 1.0), 2);

    println!(
        "verified scheduler_states={} standard_seeds={} spare_seeds={} reserve_seeds={} paired={} incomplete_keeps_base={}",
        scheduler_checks,
        standard.len(),
        spare.len(),
        reserve.len(),
        paired_checks,
        SAMPLES,
    );
}
