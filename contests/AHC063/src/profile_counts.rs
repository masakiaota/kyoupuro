#[cfg_attr(not(profile_counts), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default)]
pub struct ProfileCounters {
    pub step_calls: u64,
    pub step_ate: u64,
    pub step_bite: u64,
    pub encode_key_calls: u64,
    pub fastlane_calls: u64,
    pub fastlane_success: u64,
    pub plan_quick_calls: u64,
    pub plan_quick_expansions: u64,
    pub plan_quick_success: u64,
    pub try_recover_exact_calls: u64,
    pub try_recover_exact_steps: u64,
    pub try_recover_exact_success: u64,
    pub stage_search_calls: u64,
    pub stage_search_expansions: u64,
    pub stage_search_solution_hits: u64,
    pub bfs_calls: u64,
    pub bfs_pops: u64,
    pub bfs_success: u64,
    pub navigate_safe_calls: u64,
    pub navigate_safe_steps: u64,
    pub navigate_safe_success: u64,
    pub navigate_loose_calls: u64,
    pub navigate_loose_steps: u64,
    pub navigate_loose_success: u64,
    pub shrink_calls: u64,
    pub shrink_steps: u64,
    pub shrink_success: u64,
    pub try_target_exact_calls: u64,
    pub try_target_exact_success: u64,
    pub try_target_empty_path_calls: u64,
    pub try_target_empty_path_expansions: u64,
    pub try_target_empty_path_success: u64,
    pub collect_exact_calls: u64,
    pub collect_exact_targets: u64,
    pub collect_exact_returned: u64,
    pub collect_exact_turn_calls: u64,
    pub collect_exact_turn_targets: u64,
    pub collect_exact_turn_returned: u64,
    pub rescue_stage_calls: u64,
    pub rescue_stage_beam_inputs: u64,
    pub rescue_stage_returned: u64,
    pub trim_stage_beam_calls: u64,
    pub trim_stage_beam_inputs: u64,
    pub trim_stage_beam_returned: u64,
    pub solve_base_iters: u64,
    pub solve_base_beam_inputs: u64,
    pub solve_base_new_map_size_sum: u64,
    pub solve_base_rescue_calls: u64,
    pub solve_suffix_iters: u64,
    pub solve_suffix_beam_inputs: u64,
    pub solve_suffix_new_map_size_sum: u64,
    pub optimize_suffix_windows: u64,
    pub time_over_hits: u64,
    pub final_turns: u64,
}

#[cfg_attr(not(profile_counts), allow(dead_code))]
impl ProfileCounters {
    pub const ZERO: Self = Self {
        step_calls: 0,
        step_ate: 0,
        step_bite: 0,
        encode_key_calls: 0,
        fastlane_calls: 0,
        fastlane_success: 0,
        plan_quick_calls: 0,
        plan_quick_expansions: 0,
        plan_quick_success: 0,
        try_recover_exact_calls: 0,
        try_recover_exact_steps: 0,
        try_recover_exact_success: 0,
        stage_search_calls: 0,
        stage_search_expansions: 0,
        stage_search_solution_hits: 0,
        bfs_calls: 0,
        bfs_pops: 0,
        bfs_success: 0,
        navigate_safe_calls: 0,
        navigate_safe_steps: 0,
        navigate_safe_success: 0,
        navigate_loose_calls: 0,
        navigate_loose_steps: 0,
        navigate_loose_success: 0,
        shrink_calls: 0,
        shrink_steps: 0,
        shrink_success: 0,
        try_target_exact_calls: 0,
        try_target_exact_success: 0,
        try_target_empty_path_calls: 0,
        try_target_empty_path_expansions: 0,
        try_target_empty_path_success: 0,
        collect_exact_calls: 0,
        collect_exact_targets: 0,
        collect_exact_returned: 0,
        collect_exact_turn_calls: 0,
        collect_exact_turn_targets: 0,
        collect_exact_turn_returned: 0,
        rescue_stage_calls: 0,
        rescue_stage_beam_inputs: 0,
        rescue_stage_returned: 0,
        trim_stage_beam_calls: 0,
        trim_stage_beam_inputs: 0,
        trim_stage_beam_returned: 0,
        solve_base_iters: 0,
        solve_base_beam_inputs: 0,
        solve_base_new_map_size_sum: 0,
        solve_base_rescue_calls: 0,
        solve_suffix_iters: 0,
        solve_suffix_beam_inputs: 0,
        solve_suffix_new_map_size_sum: 0,
        optimize_suffix_windows: 0,
        time_over_hits: 0,
        final_turns: 0,
    };
}

#[cfg(profile_counts)]
struct ProfileCell(UnsafeCell<ProfileCounters>);

#[cfg(profile_counts)]
unsafe impl Sync for ProfileCell {}

#[cfg(profile_counts)]
static COUNTERS: ProfileCell = ProfileCell(UnsafeCell::new(ProfileCounters::ZERO));

#[cfg(profile_counts)]
pub fn counters_ptr() -> *mut ProfileCounters {
    COUNTERS.0.get()
}

#[cfg(profile_counts)]
pub fn dump(bin: &str) {
    let counters = unsafe { *COUNTERS.0.get() };
    eprintln!("PROFILE\tbin\t{bin}");
    macro_rules! emit {
        ($field:ident) => {
            eprintln!("PROFILE\t{}\t{}", stringify!($field), counters.$field);
        };
    }
    emit!(step_calls);
    emit!(step_ate);
    emit!(step_bite);
    emit!(encode_key_calls);
    emit!(fastlane_calls);
    emit!(fastlane_success);
    emit!(plan_quick_calls);
    emit!(plan_quick_expansions);
    emit!(plan_quick_success);
    emit!(try_recover_exact_calls);
    emit!(try_recover_exact_steps);
    emit!(try_recover_exact_success);
    emit!(stage_search_calls);
    emit!(stage_search_expansions);
    emit!(stage_search_solution_hits);
    emit!(bfs_calls);
    emit!(bfs_pops);
    emit!(bfs_success);
    emit!(navigate_safe_calls);
    emit!(navigate_safe_steps);
    emit!(navigate_safe_success);
    emit!(navigate_loose_calls);
    emit!(navigate_loose_steps);
    emit!(navigate_loose_success);
    emit!(shrink_calls);
    emit!(shrink_steps);
    emit!(shrink_success);
    emit!(try_target_exact_calls);
    emit!(try_target_exact_success);
    emit!(try_target_empty_path_calls);
    emit!(try_target_empty_path_expansions);
    emit!(try_target_empty_path_success);
    emit!(collect_exact_calls);
    emit!(collect_exact_targets);
    emit!(collect_exact_returned);
    emit!(collect_exact_turn_calls);
    emit!(collect_exact_turn_targets);
    emit!(collect_exact_turn_returned);
    emit!(rescue_stage_calls);
    emit!(rescue_stage_beam_inputs);
    emit!(rescue_stage_returned);
    emit!(trim_stage_beam_calls);
    emit!(trim_stage_beam_inputs);
    emit!(trim_stage_beam_returned);
    emit!(solve_base_iters);
    emit!(solve_base_beam_inputs);
    emit!(solve_base_new_map_size_sum);
    emit!(solve_base_rescue_calls);
    emit!(solve_suffix_iters);
    emit!(solve_suffix_beam_inputs);
    emit!(solve_suffix_new_map_size_sum);
    emit!(optimize_suffix_windows);
    emit!(time_over_hits);
    emit!(final_turns);
}

#[cfg(not(profile_counts))]
pub fn dump(_: &str) {}
#[cfg(profile_counts)]
use std::cell::UnsafeCell;
