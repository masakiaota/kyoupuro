// check_contact_sync.rs
#![allow(non_snake_case)]

const BASE_CONTACT_WEIGHT: f64 = 10.0;
const DEPARTURE_SYNC_BONUS_WEIGHT: f64 = 1.0;

fn edge_weight(S: usize, incoming_T: usize, owner_T: usize, theta: f64) -> (f64, f64) {
    let D = incoming_T - S;
    let overlap = owner_T.min(incoming_T).saturating_sub(S);
    let overlap_ratio = (overlap as f64) / (D as f64);
    let base = BASE_CONTACT_WEIGHT * overlap_ratio;
    let proximity = (-(incoming_T.abs_diff(owner_T) as f64) / theta).exp();
    let bonus = DEPARTURE_SYNC_BONUS_WEIGHT * overlap_ratio * proximity;
    (base, bonus)
}

fn main() {
    let (base_equal, bonus_equal) = edge_weight(100, 500, 500, 100.0);
    let (base_late, bonus_late) = edge_weight(100, 500, 800, 100.0);
    let (base_early, bonus_early) = edge_weight(100, 500, 450, 100.0);

    assert!((base_equal - base_late).abs() < 1e-12);
    assert!(bonus_equal > bonus_late);
    assert!(base_equal + bonus_equal > base_late + bonus_late);
    assert!(base_early < base_equal);
    assert!(bonus_early > 0.0 && bonus_early <= DEPARTURE_SYNC_BONUS_WEIGHT);

    for owner_T in 101..=1_000 {
        let (_, bonus) = edge_weight(100, 500, owner_T, 100.0);
        assert!(bonus.is_finite());
        assert!((0.0..=DEPARTURE_SYNC_BONUS_WEIGHT).contains(&bonus));
    }

    println!("contact sync formula: ok");
}
