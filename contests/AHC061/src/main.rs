use std::process::exit;

use fixedbitset::FixedBitSet;
use itertools::Itertools;
use proconio::input_interactive;

const TIME_LIMIT_SEC: f64 = 1.97;
const N: usize = 10;
const T: usize = 100;
const V_AVG: f32 = 1000.0;
const W_MIN: f32 = 0.3;
const W_MAX: f32 = 1.0;
const EPS_MIN: f32 = 0.1;
const EPS_MAX: f32 = 0.5;
const DXDY: [(usize, usize); 4] = [(!0, 0), (1, 0), (0, !0), (0, 1)];
const CENTER: f32 = (N - 1) as f32 / 2.0;
const N1: usize = N + 1;
const IN_GRID: u128 = 0x1ffbff7feffdffbff7feffdffbff;

const TOP_DIST_WEIGHT: f32 = 15.0;
const RAW_SCORE_DECAY: f32 = 0.9;
const CONTRARY_BASE: f32 = 500.0;
const ATTACK_BONUS: f32 = 3.5;
const VALUE_PENALTY: f32 = 5e-2;
const SURROUNDS_BONUS_U2: f32 = 0.1;
const SURROUNDS_BONUS: f32 = 0.5;
const M2_LV2_BONUS: f32 = 1.0;
const ARTICULATION_BONUS: f32 = 0.25;

const DEST_TIME_RATE: f64 = 0.05;
const DEST_TIME: f64 = DEST_TIME_RATE * TIME_LIMIT_SEC;
const MC_TIME_PER_TURN_SEC: f64 =
    (1.0 - MCMC_TIME_RATE - DEST_TIME_RATE) * TIME_LIMIT_SEC / T as f64;
const MC_TIME_MIN: f64 = MC_TIME_PER_TURN_SEC * 0.5;
const MC_TIME_MAX: f64 = MC_TIME_PER_TURN_SEC * 1.5;

const NUM_SAMPLES_FIRST: usize = 10;
const NUM_SAMPLES_LAST: usize = 10;
const T0: f64 = 0.5;
const T1: f64 = 0.0;
const SCHED_TYPE: SchedulerType = SchedulerType::Linear;
const MCMC_TIME_RATE: f64 = 0.05;
const MCMC_TIME_PER_TURN_SEC: f64 = MCMC_TIME_RATE * TIME_LIMIT_SEC / T as f64;
const R_RESAMPLE_W: usize = 4;
const R_RESAMPLE_EPS: usize = 1 + R_RESAMPLE_W;
const RESAMPLE_W_INV: usize = 8;

fn main() {
    get_time_sec();

    let input = read_input();
    let mut solver = Solver::new(input);
    solver.solve();

    exit(0);
}

#[derive(Debug, Clone)]
struct Solver {
    m: usize,
    lv_max: i32,
    env: Environment,
    num_sim_turns: usize,
}

impl Solver {
    fn new(input: Input) -> Self {
        let env = Environment::new(&input);
        Self {
            m: input.m,
            lv_max: input.lv_max,
            env,
            num_sim_turns: 0,
        }
    }

    fn solve(&mut self) {
        let mut state = State::new(&self.env);

        if self.m > 2 {
            state.destinations = self.decide_destinations(&state);
        }

        for turn in 0..T {
            if turn > 0 {
                self.env.sample_params(turn);
            }
            for p in 1..self.m {
                state.update_ai_policy(p, &self.env.params[p], &self.env);
            }
            let (x, y) = self.action_policy(&state);
            println!("{} {}", x, y);

            #[cfg(not(feature = "no_interact"))]
            input_interactive! {
                actions: [(usize, usize); self.m],
            }
            #[cfg(feature = "no_interact")]
            let actions = {
                let mut actions = vec![(0, 0); self.m];
                actions[0] = (x, y);
                for p in 1..self.m {
                    let candidates = state.actions[p];
                    let i = self.env.rng.gen_range(0, candidates.count_ones() as usize);
                    let v = select128(candidates, i);
                    actions[p] = (v / N1, v % N1);
                }
                actions
            };

            self.add_results(&actions, &state);
            state.apply_actions(&actions, &self.env);

            #[cfg(not(feature = "no_interact"))]
            state.read_state();
        }
        // dbg!(self.num_sim_turns);
        // self.env.sched.print_log();
    }

    fn add_results(&mut self, actions: &[(usize, usize)], state: &State) {
        for p in 1..self.m {
            let num_actions = state.actions[p].count_ones() as usize;
            if let Some(i) = state.policies[p]
                .bests
                .iter()
                .position(|&pos| pos == actions[p])
            {
                // may be greedy action
                let res = GreedyResults::new(state.policies[p].evals, i, num_actions);
                self.env.add_greedy_result(p, res);
            } else {
                // random action
                self.env.add_random_result(p, num_actions);
            }
        }
    }

    fn action_policy(&mut self, state: &State) -> (usize, usize) {
        let fixed_action = state.get_fixed_action(&self.env);
        if fixed_action != (!0, !0) {
            return fixed_action;
        }
        let mut candidates_popping = state.promising_actions(&self.env);
        let mut candidates = Vec::with_capacity(candidates_popping.count_ones() as usize);
        while candidates_popping != 0 {
            let v = pop128(&mut candidates_popping);
            let (x, y) = (v / N1, v % N1);
            let profit = state.evaluate_my_action(x, y, &self.env);
            candidates.push((-profit, x, y));
        }
        candidates.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut candidates = candidates.into_iter().map(|(_, x, y)| (x, y)).collect_vec();
        if candidates.len() == 1 {
            return candidates[0];
        }
        if self.m == 2 {
            candidates.truncate(8);
        }
        let first_num_cands = candidates.len();

        // eprintln!("turn: {}", state.turn);
        // eprintln!("cand: {:?} {}", candidates[0], state.evaluate_my_action(candidates[0].0, candidates[0].1, &self.env));

        // Monte Carlo simulation
        let mut scores = vec![0.0; candidates.len()];
        let num_samples = self.env.params[1].len();
        let playout_turns = 20;
        let start_time = get_time_sec();
        let end_time = (TIME_LIMIT_SEC * (state.turn + 1) as f64 / T as f64)
            .clamp(start_time + MC_TIME_MIN, start_time + MC_TIME_MAX);
        if end_time > TIME_LIMIT_SEC {
            return candidates[0];
        }
        for sim in 0.. {
            if sim > 0 && sim % num_samples == 0 {
                for p in 1..self.m {
                    self.env.rng.shuffle(&mut self.env.params[p]);
                }
            }
            let params = (0..self.m)
                .map(|p| self.env.params[p][sim % num_samples].clone())
                .collect_vec();
            let simulation_setting = SimulationSetting::new(
                self.m,
                state.turn,
                T.min(state.turn + playout_turns),
                params,
                &mut self.env.rng,
            );
            for (i, &(x, y)) in candidates.iter().enumerate() {
                let mut state = state.clone();
                state.end_turn = simulation_setting.end_turn;
                // simulate 1 turn with the candidate action
                let mut actions = vec![(0, 0); self.m];
                actions[0] = (x, y);
                state.set_ai_actions(&mut actions, &simulation_setting);
                state.apply_actions(&actions, &self.env);
                // play out
                state = self.simulate(state, &simulation_setting);
                scores[i] += state.evaluate();
            }
            self.num_sim_turns += candidates.len() * simulation_setting.num_simulation_turns();

            // stop if time is up
            let now_time = get_time_sec();
            if now_time >= end_time {
                break;
            }
            // progressively reduce the number of candidates
            let progress = (now_time - start_time) / (end_time - start_time);
            let curr_num_cands =
                Self::successive_halving(first_num_cands, candidates.len(), progress);
            while candidates.len() > curr_num_cands {
                let worst = (0..scores.len())
                    .min_by(|&i, &j| scores[i].partial_cmp(&scores[j]).unwrap())
                    .unwrap();
                candidates.remove(worst);
                scores.remove(worst);
            }
        }
        let best = (0..scores.len())
            .max_by(|&i, &j| scores[i].partial_cmp(&scores[j]).unwrap())
            .unwrap();
        // eprintln!("best: {:?} {}", candidates[best], state.evaluate_my_action(candidates[best].0, candidates[best].1, &self.env));
        candidates[best]
    }

    fn successive_halving(first_num_cands: usize, curr_num_cands: usize, progress: f64) -> usize {
        let l = first_num_cands as f64;
        let r = 1.0f64;
        let m = l.powf(1.0 - progress) + r.powf(progress);
        let m = (m.ceil() as usize).clamp(2, curr_num_cands);
        if 2 * m < curr_num_cands {
            m
        } else {
            curr_num_cands
        }
    }

    fn greedy(&self, state: &mut State) -> (usize, usize) {
        for p in 1..self.m {
            state.update_ai_policy(p, &self.env.params[p], &self.env);
        }
        let fixed_action = state.get_fixed_action(&self.env);
        if fixed_action != (!0, !0) {
            return fixed_action;
        }
        let mut best_pos = (!0, !0);
        let mut max_profit = f32::MIN;
        let mut my_actions = state.useful_actions();
        while my_actions != 0 {
            let v = pop128(&mut my_actions);
            let (x, y) = (v / N1, v % N1);
            let profit = state.evaluate_my_action(x, y, &self.env);
            if max_profit.chmax(profit) {
                best_pos = (x, y);
            }
        }
        best_pos
    }

    fn decide_destinations(&mut self, state: &State) -> u128 {
        let mut candidates = vec![0];
        // corners
        candidates.push(1 << (0 * N1 + 0));
        candidates.push(1 << (0 * N1 + N - 1));
        candidates.push(1 << ((N - 1) * N1 + 0));
        candidates.push(1 << ((N - 1) * N1 + N - 1));
        // edges
        // up
        let mut b = 0;
        for y in 0..N {
            b |= 1 << (0 * N1 + y);
        }
        candidates.push(b);
        // down
        let mut b = 0;
        for y in 0..N {
            b |= 1 << ((N - 1) * N1 + y);
        }
        candidates.push(b);
        // left
        let mut b = 0;
        for x in 0..N {
            b |= 1 << (x * N1 + 0);
        }
        candidates.push(b);
        // right
        let mut b = 0;
        for x in 0..N {
            b |= 1 << (x * N1 + N - 1);
        }
        candidates.push(b);

        // top position
        if self.m <= 6 {
            let (top_x, top_y) = self.env.top_pos;
            candidates.push(1 << (top_x * N1 + top_y));
        }

        candidates.retain(|&destinations| destinations & state.owners128[0] == 0);
        let first_num_cands = candidates.len();

        // Monte Carlo simulation
        let mut scores = vec![0.0; candidates.len()];
        let num_samples = self.env.params[1].len();
        let start_time = 0.0;
        let end_time = DEST_TIME;
        for sim in 0.. {
            if sim > 0 && sim % num_samples == 0 {
                for p in 1..self.m {
                    self.env.rng.shuffle(&mut self.env.params[p]);
                }
            }
            let params = (0..self.m)
                .map(|p| self.env.params[p][sim % num_samples].clone())
                .collect_vec();
            let simulation_setting =
                SimulationSetting::new(self.m, state.turn, T, params, &mut self.env.rng);
            for (i, &destinations) in candidates.iter().enumerate() {
                let mut state = state.clone();
                state.destinations = destinations;
                // play out
                state = self.simulate(state, &simulation_setting);
                scores[i] += state.evaluate();
            }
            self.num_sim_turns += candidates.len() * simulation_setting.num_simulation_turns();

            // stop if time is up
            let now_time = get_time_sec();
            if now_time >= end_time {
                break;
            }
            // progressively reduce the number of candidates
            let progress = (now_time - start_time) / (end_time - start_time);
            let curr_num_cands =
                Self::successive_halving(first_num_cands, candidates.len(), progress);
            while candidates.len() > curr_num_cands {
                let worst = (0..scores.len())
                    .min_by(|&i, &j| scores[i].partial_cmp(&scores[j]).unwrap())
                    .unwrap();
                candidates.remove(worst);
                scores.remove(worst);
            }
        }
        let best = (0..scores.len())
            .max_by(|&i, &j| scores[i].partial_cmp(&scores[j]).unwrap())
            .unwrap();
        candidates[best]
    }

    fn simulate(&self, mut state: State, simulation_setting: &SimulationSetting) -> State {
        while state.turn < simulation_setting.end_turn {
            let (x, y) = self.greedy(&mut state);
            let mut actions = vec![(0, 0); self.m];
            actions[0] = (x, y);
            state.set_ai_actions(&mut actions, simulation_setting);
            state.apply_actions(&actions, &self.env);
        }
        state
    }
}

#[derive(Debug, Clone)]
struct State {
    m: usize,
    lv_max: i32,
    pos: Vec<(usize, usize)>,
    owners: [[usize; N]; N],
    owners128: Vec<u128>, // bitboard of owned cells for each player
    no_owners: u128,      // bitboard of cells with no owner
    levels: [[i32; N]; N],
    level_maxs: u128, // bitboard of cells with level == lv_max
    scores: Vec<f32>,
    turn: usize,
    progress: f32,
    rprogress: f32,
    best_ai: usize,
    connected: Vec<u128>,   // bitboard of connected cells for each player
    any_ai_connected: u128, // connected[1] | ... | connected[m-1]
    actions: Vec<u128>,
    any_ai_actions: u128,      // actions[1] | ... | actions[m-1]
    conflict_ai_actions: u128, // actions that can be taken by 2 or more AIs
    policies: Vec<AiPolicy>,
    best_ai_ex_score: f32, // expected next score of the best AI
    avg_raw_score: f32,
    destinations: u128,
    end_turn: usize,
}

impl State {
    fn new(env: &Environment) -> Self {
        let m = env.m;
        let mut owners = [[!0; N]; N];
        let mut levels = [[0; N]; N];
        let mut level_maxs = 0;
        let mut scores = vec![0.0; m];
        let mut owners128 = vec![0; m];
        let mut no_owners = IN_GRID;
        let mut connected = vec![0; m];
        for p in 0..m {
            let (x, y) = env.initial_pos[p];
            owners[x][y] = p;
            levels[x][y] = 1;
            if env.lv_max == 1 {
                set128(&mut level_maxs, x * N1 + y, true);
            }
            set128(&mut owners128[p], x * N1 + y, true);
            set128(&mut no_owners, x * N1 + y, false);
            set128(&mut connected[p], x * N1 + y, true);
            scores[p] += env.values[x][y];
        }
        let mut res = Self {
            m,
            lv_max: env.lv_max,
            pos: env.initial_pos.clone(),
            owners,
            owners128,
            no_owners,
            levels,
            level_maxs,
            scores,
            turn: 0,
            progress: 0.0,
            rprogress: 1.0,
            best_ai: 1,
            connected,
            any_ai_connected: 0,
            actions: vec![0; m],
            any_ai_actions: 0,
            conflict_ai_actions: 0,
            policies: vec![AiPolicy::default(); m],
            best_ai_ex_score: 0.0,
            avg_raw_score: 0.0,
            destinations: 0,
            end_turn: T,
        };
        res.update_data(env);
        res
    }

    /// Called after applying actions
    fn read_state(&self) {
        input_interactive! {
            pos: [(usize, usize); self.m],
            owners: [[i32; N]; N],
            levels: [[i32; N]; N],
        }
        let owners: [[usize; N]; N] = owners
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|x| x as usize)
                    .collect_vec()
                    .try_into()
                    .unwrap()
            })
            .collect_vec()
            .try_into()
            .unwrap();
        let levels: [[i32; N]; N] = levels
            .into_iter()
            .map(|row| row.try_into().unwrap())
            .collect_vec()
            .try_into()
            .unwrap();
        debug_assert_eq!(self.pos, pos);
        debug_assert_eq!(self.owners, owners);
        debug_assert_eq!(self.levels, levels);
    }

    fn raw_score(&self) -> f32 {
        1e5 * (1.0 + self.relative_score()).log2()
    }

    fn relative_score(&self) -> f32 {
        self.scores[0] / self.scores[self.best_ai]
    }

    fn evaluate(&self) -> f32 {
        match self.eval_strategy_type() {
            StrategyType::Contrary => self.owners128[0].count_ones() as f32,
            StrategyType::Absolute => self.evaluate_absolute(),
            StrategyType::Relative => self.avg_raw_score,
        }
    }

    fn evaluate_absolute(&self) -> f32 {
        self.scores[0]
            - self.progress * self.scores[self.best_ai]
            - self.rprogress * self.scores[1..].iter().sum::<f32>() / (self.m - 1) as f32
    }

    fn eval_strategy_type(&self) -> StrategyType {
        if self.m == 2 {
            StrategyType::Relative
        } else if self.lv_max == 1 {
            if self.end_turn < 95 {
                StrategyType::Contrary
            } else {
                StrategyType::Relative
            }
        } else {
            if self.end_turn < 95 {
                StrategyType::Absolute
            } else {
                StrategyType::Relative
            }
        }
    }

    fn update_data(&mut self, env: &Environment) {
        self.best_ai = (1..self.m)
            .max_by(|&p, &q| self.scores[p].partial_cmp(&self.scores[q]).unwrap())
            .unwrap();
        self.update_actions();
        self.update_ai_evals(env);
        self.update_avg_score();
        self.update_destinations();
    }

    fn update_actions(&mut self) {
        let mut pos128 = 0;
        for p in 0..self.m {
            let (x, y) = self.pos[p];
            set128(&mut pos128, x * N1 + y, true);
        }
        for p in 0..self.m {
            // BFS
            let (sx, sy) = self.pos[p];
            debug_assert_eq!(self.owners[sx][sy], p);
            let mut b = if self.connected[p] == self.owners128[p] {
                self.connected[p]
            } else {
                let b = bfs128(sx, sy, self.owners128[p]);
                self.connected[p] = b;
                b
            };
            b = expand128(b) & (!pos128 | (1 << (sx * N1 + sy)));
            self.actions[p] = b;
        }
        self.any_ai_connected = 0;
        for p in 1..self.m {
            self.any_ai_connected |= self.connected[p];
        }
        self.any_ai_actions = 0;
        self.conflict_ai_actions = 0;
        for p in 1..self.m {
            self.conflict_ai_actions |= self.any_ai_actions & self.actions[p];
            self.any_ai_actions |= self.actions[p];
        }
    }

    fn update_ai_evals(&mut self, env: &Environment) {
        for p in 1..self.m {
            self.policies[p] = self.create_ai_eval(p, self.actions[p], env);
        }
    }

    fn create_ai_eval(&self, p: usize, actions: u128, env: &Environment) -> AiPolicy {
        debug_assert!(1 <= p && p < self.m);
        debug_assert!(actions != 0);
        let mut evals = [f32::MIN; 4];
        let mut bests = [(!0, !0); 4];
        let defense_actions = actions & self.owners128[p] & self.level_maxs;
        let mut actions_popping = actions & !defense_actions;
        if actions_popping == 0 {
            actions_popping = actions;
        }
        while actions_popping != 0 {
            let v = pop128(&mut actions_popping);
            let (x, y) = (v / N1, v % N1);
            let value = env.values[x][y];
            match self.get_action_type(x, y, p) {
                ActionType::Occupy => {
                    if evals[0].chmax(value) {
                        bests[0] = (x, y);
                    }
                }
                ActionType::Reinforce => {
                    if evals[1].chmax(value) {
                        bests[1] = (x, y);
                    }
                }
                ActionType::Defense => {
                    if evals[1].chmax(0.0) {
                        bests[1] = (x, y);
                    }
                }
                ActionType::Steal => {
                    if evals[2].chmax(value) {
                        bests[2] = (x, y);
                    }
                }
                ActionType::Attack => {
                    if evals[3].chmax(value) {
                        bests[3] = (x, y);
                    }
                }
            }
        }
        AiPolicy::new(actions, evals, bests)
    }

    fn update_turn(&mut self) {
        self.turn += 1;
        self.progress = self.turn as f32 / T as f32;
        self.rprogress = 1.0 - self.progress;
    }

    fn update_destinations(&mut self) {
        if self.destinations & self.owners128[0] != 0 {
            self.destinations = 0;
        }
    }

    fn update_ai_policy(&mut self, p: usize, params: &[PlayerParam], env: &Environment) {
        debug_assert!(1 <= p && p < self.m);
        self.policies[p].update_prob(params);
        if p == self.best_ai {
            self.update_best_ai_ex_score(env);
        }
    }

    fn update_best_ai_ex_score(&mut self, env: &Environment) {
        self.best_ai_ex_score = self.scores[self.best_ai];
        let mut actions_popping = self.actions[self.best_ai];
        actions_popping &= !(self.owners128[self.best_ai] & self.level_maxs);
        if self.lv_max >= 2 {
            actions_popping &= !self.level_maxs;
        }
        while actions_popping != 0 {
            let v = pop128(&mut actions_popping);
            let (x, y) = (v / N1, v % N1);
            // assume action is successful
            let owner = self.owners[x][y];
            let value = env.values[x][y];
            let profit = if owner == !0 {
                value
            } else if owner == self.best_ai {
                value
            } else if self.levels[x][y] == 1 {
                value
            } else {
                0.0
            };
            let prob = self.policies[self.best_ai].chosen_prob(x, y);
            self.best_ai_ex_score += profit * prob;
        }
    }

    fn update_avg_score(&mut self) {
        self.avg_raw_score *= RAW_SCORE_DECAY;
        self.avg_raw_score += (1.0 - RAW_SCORE_DECAY) * self.raw_score();
    }

    fn set_ai_actions(
        &self,
        actions: &mut [(usize, usize)],
        simulation_setting: &SimulationSetting,
    ) {
        debug_assert_eq!(actions.len(), self.m);
        for p in 1..self.m {
            actions[p] = if simulation_setting.get_is_random(p, self.turn) {
                let random_r = simulation_setting.get_random_r(p, self.turn);
                let mut candidates = self.actions[p];
                let num_candidates = candidates.count_ones();
                let mut i =
                    ((random_r * num_candidates as f32) as usize).min(num_candidates as usize - 1);
                if self.m == 2 || self.turn >= 95 {
                    // sort candidate actions by luckiness
                    let (x0, y0) = actions[0];
                    let mut ai_lv_max = candidates & self.owners128[p] & self.level_maxs;
                    set128(&mut ai_lv_max, x0 * N1 + y0, false);
                    ai_lv_max |= candidates & self.owners128[0] & (1 << (x0 * N1 + y0));
                    let num_ai_lv_max = ai_lv_max.count_ones() as usize;
                    if i < num_ai_lv_max {
                        let v = select128(ai_lv_max, i);
                        (v / N1, v % N1)
                    } else {
                        i -= num_ai_lv_max;
                        candidates &= !ai_lv_max;
                        let mut ai_or_no_owner = candidates & !self.owners128[0];
                        set128(&mut ai_or_no_owner, x0 * N1 + y0, false);
                        let num_ai_or_no_owner = ai_or_no_owner.count_ones() as usize;
                        if i < num_ai_or_no_owner {
                            let v = select128(ai_or_no_owner, i);
                            (v / N1, v % N1)
                        } else {
                            i -= num_ai_or_no_owner;
                            candidates &= !ai_or_no_owner;
                            let v = select128(candidates, i);
                            (v / N1, v % N1)
                        }
                    }
                } else {
                    let v = select128(candidates, i);
                    (v / N1, v % N1)
                }
            } else {
                self.policies[p].get_greedy_action(&simulation_setting.params[p])
            };
        }
    }

    fn no_ai_prob(&self, x: usize, y: usize) -> f32 {
        if !get128(self.any_ai_actions, x * N1 + y) {
            return 1.0;
        }
        let mut prob = 1.0;
        for p in 1..self.m {
            prob *= 1.0 - self.policies[p].chosen_prob(x, y);
        }
        prob
    }

    fn conflict_ai_prob(&self, x: usize, y: usize) -> f32 {
        if !get128(self.conflict_ai_actions, x * N1 + y) {
            return 0.0;
        }
        let mut prob_zero = 1.0;
        let mut prob_one = 0.0;
        for p in 1..self.m {
            if get128(self.actions[p], x * N1 + y) {
                let pi = self.policies[p].chosen_prob(x, y);
                prob_one = prob_one * (1.0 - pi) + prob_zero * pi;
                prob_zero *= 1.0 - pi;
            }
        }
        (1.0 - prob_zero - prob_one).max(0.0)
    }

    fn apply_actions(&mut self, actions: &[(usize, usize)], env: &Environment) {
        debug_assert!(actions
            .iter()
            .enumerate()
            .all(|(p, pos)| get128(self.actions[p], pos.0 * N1 + pos.1)));

        // conflict resolution
        let mut no_conflict = 0;
        let mut conflict = 0;
        for p in 0..self.m {
            let (vx, vy) = actions[p];
            let v = vx * N1 + vy;
            if !get128(no_conflict, v) {
                set128(&mut no_conflict, v, true);
            } else {
                set128(&mut conflict, v, true);
            }
        }
        // territory update
        for p in 0..self.m {
            let (x, y) = actions[p];
            let owner = self.owners[x][y];
            if owner != p && get128(conflict, x * N1 + y) {
                // conflict
                continue;
            }
            if owner == !0 {
                // occupation
                self.pos[p] = (x, y);
                self.owners[x][y] = p;
                set128(&mut self.owners128[p], x * N1 + y, true);
                set128(&mut self.no_owners, x * N1 + y, false);
                self.levels[x][y] = 1;
                if self.lv_max == 1 {
                    set128(&mut self.level_maxs, x * N1 + y, true);
                }
                self.scores[p] += env.values[x][y];
            } else if owner == p {
                // reinforcement
                self.pos[p] = (x, y);
                if self.levels[x][y] < self.lv_max {
                    self.levels[x][y] += 1;
                    if self.levels[x][y] == self.lv_max {
                        set128(&mut self.level_maxs, x * N1 + y, true);
                    }
                    self.scores[p] += env.values[x][y];
                }
            } else {
                // attack
                if self.levels[x][y] == 1 {
                    self.pos[p] = (x, y);
                    self.owners[x][y] = p;
                    set128(&mut self.owners128[owner], x * N1 + y, false);
                    set128(&mut self.owners128[p], x * N1 + y, true);
                    self.scores[p] += env.values[x][y];
                } else {
                    if self.levels[x][y] == self.lv_max {
                        set128(&mut self.level_maxs, x * N1 + y, false);
                    }
                    self.levels[x][y] -= 1;
                }
                self.scores[owner] -= env.values[x][y];
            }
        }
        self.update_data(env);
        self.update_turn();
    }

    fn is_articulation(&self, x: usize, y: usize, env: &Environment) -> bool {
        let p = self.owners[x][y];
        if p == !0 {
            return false;
        }
        let b = self.owners128[p];
        let v = x * N1 + y;
        let mut neighbors = 0;
        if x > 0 {
            if y == 0 {
                neighbors |= (b << 1) as usize & 0b111;
            } else {
                neighbors |= (b >> (v - N1 - 1)) as usize & 0b111;
            }
        }
        if v == 0 {
            neighbors |= ((b << 1) as usize & 0b101) << 3;
        } else {
            neighbors |= ((b >> (v - 1)) as usize & 0b101) << 3;
        }
        if x < N - 1 {
            neighbors |= ((b >> (v + N1 - 1)) as usize & 0b111) << 6;
        }
        !env.connected3x3[neighbors]
    }

    fn count_my_surrounds(&self, x: usize, y: usize) -> usize {
        let mut cnt = 0;
        for &(dx, dy) in &DXDY {
            let nx = x.wrapping_add(dx);
            let ny = y.wrapping_add(dy);
            if nx >= N || ny >= N || self.owners[nx][ny] == 0 {
                cnt += 1;
            }
        }
        cnt
    }

    #[allow(dead_code)]
    fn be_attacked_profit(&self, x: usize, y: usize) -> f32 {
        let profit = if self.owners[x][y] == 0 {
            0
        } else {
            let mut me = self.owners128[0];
            let mut profit = (me & self.any_ai_actions).count_ones() as i32;
            set128(&mut me, x * N1 + y, true);
            let mut ai = self.any_ai_connected;
            set128(&mut ai, x * N1 + y, false);
            let ai_actions = expand128(ai);
            profit -= (me & ai_actions).count_ones() as i32;
            profit
        };
        (profit + 1) as f32
    }

    fn get_fixed_action(&self, env: &Environment) -> (usize, usize) {
        if self.m == 2 && expand128(self.connected[0]) & self.owners128[1] == 0 {
            let mut b = self.owners128[1];
            while b & self.actions[0] == 0 {
                b = expand128(b);
            }
            let mut candidates = b & self.actions[0];
            let mut best_pos = (!0, !0);
            let mut best_profit = f32::MIN;
            while candidates != 0 {
                let v = pop128(&mut candidates);
                let (x, y) = (v / N1, v % N1);
                let profit = self.evaluate_my_action(x, y, env);
                if best_profit.chmax(profit) {
                    best_pos = (x, y);
                }
            }
            return best_pos;
        }
        if self.destinations != 0 {
            let mut best_pos = (!0, !0);
            let mut best_profit = f32::MIN;
            let mut actions_popping = self.approach_destination_actions();
            while actions_popping != 0 {
                let v = pop128(&mut actions_popping);
                let (x, y) = (v / N1, v % N1);
                if self.levels[x][y] >= 3 {
                    continue;
                }
                let no_ai_prob = self.no_ai_prob(x, y);
                if no_ai_prob < 0.5 {
                    continue;
                }
                let profit = no_ai_prob;
                if best_profit.chmax(profit) {
                    best_pos = (x, y);
                }
            }
            return best_pos;
        }
        (!0, !0)
    }

    fn approach_destination_actions(&self) -> u128 {
        let mut b = self.destinations;
        while b & self.actions[0] == 0 {
            b = expand128(b);
        }
        if b & self.owners128[0] != 0 {
            0
        } else {
            b & self.actions[0]
        }
    }

    fn get_strategy_type(&self) -> StrategyType {
        if self.turn + 5 >= self.end_turn {
            return self.eval_strategy_type();
        }
        if self.m == 2 {
            StrategyType::Relative
        } else if self.lv_max == 1 {
            if self.turn < 95 {
                StrategyType::Contrary
            } else {
                StrategyType::Relative
            }
        } else {
            if self.turn < 95 {
                StrategyType::Absolute
            } else {
                StrategyType::Relative
            }
        }
    }

    fn evaluate_my_action(&self, x: usize, y: usize, env: &Environment) -> f32 {
        match self.get_strategy_type() {
            StrategyType::Contrary => self.contrary_profit(x, y, env),
            StrategyType::Absolute => self.absolute_profit(x, y, env),
            StrategyType::Relative => self.relative_profit(x, y, env),
        }
    }

    fn contrary_profit(&self, x: usize, y: usize, env: &Environment) -> f32 {
        let mut profit = 0.0;
        let no_ai_prob = self.no_ai_prob(x, y);
        let mut value = CONTRARY_BASE + env.distance_from_top(x, y);
        if self.lv_max == 1 {
            value += VALUE_PENALTY * self.rprogress * (2000.0 - env.values[x][y]).max(0.0);
        }
        let owner = self.owners[x][y];
        if self.lv_max >= 2 {
            if self.count_my_surrounds(x, y) >= 2 {
                value *= 1.0 + SURROUNDS_BONUS_U2 * self.rprogress;
            }
        }
        if owner == !0 {
            profit += value * no_ai_prob;
        } else if owner == 0 {
            if self.levels[x][y] < self.lv_max {
                value *= (env.values[x][y] / V_AVG).powf(self.progress);
                profit += value * (2.0 - no_ai_prob - self.conflict_ai_prob(x, y));
            } else {
                profit += value * (1.0 - no_ai_prob - self.conflict_ai_prob(x, y));
            }
        } else if self.levels[x][y] == 1 {
            profit += value * no_ai_prob;
        } else {
            let r = 1.0 / (self.m - 1) as f32;
            profit += r * value * no_ai_prob;
        }
        profit
    }

    fn absolute_profit(&self, x: usize, y: usize, env: &Environment) -> f32 {
        let mut profit = 0.0;
        let no_ai_prob = self.no_ai_prob(x, y);
        let owner = self.owners[x][y];
        let mut value = env.values[x][y];

        if self.count_my_surrounds(x, y) >= 2 {
            value *= 1.0 + SURROUNDS_BONUS * self.rprogress;
        }
        if self.pos[0] == (x, y) {
            if self.lv_max >= 3 {
                value *= 0.75;
            }
        }
        if owner == !0 {
            profit += value * (no_ai_prob + 0.2 * self.rprogress);
            let best_ai_prob = self.policies[self.best_ai].chosen_prob(x, y);
            profit += value
                * (self.progress * best_ai_prob
                    + self.rprogress * (1.0 - no_ai_prob) / (self.m - 1) as f32);
        } else if owner == 0 {
            if self.levels[x][y] < self.lv_max {
                let mut r = 2.0 - no_ai_prob - self.conflict_ai_prob(x, y);
                if self.levels[x][y] == 1 {
                    if no_ai_prob < 1.0 - 1e-3 {
                        r += 0.5 * self.rprogress;
                        let best_ai_prob = self.policies[self.best_ai].chosen_prob(x, y);
                        profit += value
                            * (self.progress * best_ai_prob
                                + self.rprogress * (1.0 - no_ai_prob) / (self.m - 1) as f32);
                    };
                }
                profit += r * value;
            } else {
                // profit += value * (1.0 - no_ai_prob - self.conflict_ai_prob(x, y));
            }
        } else if self.levels[x][y] == 1 {
            let mut r = self.rprogress / (self.m - 1) as f32;
            if owner == self.best_ai {
                r += self.progress
            }
            profit += (1.0 + r) * value * no_ai_prob;
        } else {
            let mut r = self.rprogress / (self.m - 1) as f32;
            if owner == self.best_ai {
                r += self.progress;
            }
            r *= ATTACK_BONUS / self.levels[x][y] as f32;
            profit += r * value * no_ai_prob;
        }
        profit
    }

    fn relative_profit(&self, x: usize, y: usize, env: &Environment) -> f32 {
        let my_score = self.scores[0];
        let no_ai_prob = self.no_ai_prob(x, y);
        let any_ai_prob = 1.0 - no_ai_prob;
        let owner = self.owners[x][y];
        let mut value = env.values[x][y];
        if self.m == 2 && self.lv_max >= 2 {
            value = 1000.0 * self.rprogress + value * self.progress;
        }
        let profit = if owner == !0 {
            let mut profit = (my_score + value) / my_score;
            profit = any_ai_prob + no_ai_prob * profit;
            let best_ai_prob = self.policies[self.best_ai].chosen_prob(x, y);
            let ai_score = self.scores[self.best_ai];
            profit *= (1.0 - best_ai_prob) + best_ai_prob * ((ai_score + value) / ai_score);
            profit
        } else if owner == 0 {
            let mut profit = if self.levels[x][y] < self.lv_max {
                (my_score + (2.0 - no_ai_prob - self.conflict_ai_prob(x, y)) * value) / my_score
            } else {
                (my_score + (1.0 - no_ai_prob - self.conflict_ai_prob(x, y)) * value) / my_score
            };
            if self.levels[x][y] == 1 {
                let mut best_ai_prob = self.policies[self.best_ai].chosen_prob(x, y);
                if self.lv_max >= 2 && self.m == 2 {
                    best_ai_prob *= 1.0 + self.rprogress * M2_LV2_BONUS;
                    best_ai_prob.chmin(1.0);
                }
                let ai_score = self.scores[self.best_ai];
                profit *= (1.0 - best_ai_prob) + best_ai_prob * ((ai_score + value) / ai_score);
            }
            profit
        } else if self.levels[x][y] == 1 {
            let mut profit = (my_score + value) / my_score;
            if owner == self.best_ai {
                profit *= self.best_ai_ex_score / (self.best_ai_ex_score - value);
            }
            if self.m == 2 && no_ai_prob < 1.0 - 1e-3 {
                if self.is_articulation(x, y, env) {
                    profit *= 1.0 + self.rprogress * ARTICULATION_BONUS;
                }
            }
            any_ai_prob + no_ai_prob * profit
        } else {
            let profit = if owner == self.best_ai {
                self.best_ai_ex_score / (self.best_ai_ex_score - value)
            } else {
                1.0
            };
            any_ai_prob + no_ai_prob * profit
        };
        profit
    }

    fn useful_actions(&self) -> u128 {
        let mut useful =
            self.actions[0] & !(self.owners128[0] & self.level_maxs & !self.any_ai_actions);
        if useful == 0 {
            useful = self.actions[0];
        }
        useful
    }

    fn promising_actions(&self, env: &Environment) -> u128 {
        let mut promising = self.useful_actions();

        // Remove no defense reinforce actions and no expansion occupy actions
        let mut remove_candidates =
            promising & self.owners128[0] & !self.any_ai_actions & !self.level_maxs;
        let mut no_expansions = promising & self.no_owners & !self.any_ai_actions;
        while no_expansions != 0 {
            let v = pop128(&mut no_expansions);
            let neighbors = expand128(1 << v);
            if neighbors & self.actions[0] == neighbors {
                // no expansion possible after this action
                set128(&mut remove_candidates, v, true);
            }
        }
        if remove_candidates != 0 {
            let mut best_action = 0;
            let mut best_value = f32::MIN;
            let mut actions_popping = remove_candidates;
            while actions_popping != 0 {
                let v = pop128(&mut actions_popping);
                let (x, y) = (v / N1, v % N1);
                let value = self.evaluate_my_action(x, y, env);
                if best_value.chmax(value) {
                    best_action = 1 << (x * N1 + y);
                }
            }
            remove_candidates &= !best_action;
            promising &= !remove_candidates;
        }

        if promising == 0 {
            promising = self.actions[0];
        }
        promising
    }

    fn get_action_type(&self, x: usize, y: usize, p: usize) -> ActionType {
        let owner = self.owners[x][y];
        if owner == !0 {
            ActionType::Occupy
        } else if owner == p {
            if self.levels[x][y] < self.lv_max {
                ActionType::Reinforce
            } else {
                ActionType::Defense
            }
        } else if self.levels[x][y] == 1 {
            ActionType::Steal
        } else {
            ActionType::Attack
        }
    }
}

enum StrategyType {
    Contrary,
    Absolute,
    Relative,
}

enum ActionType {
    Occupy,
    Reinforce,
    Defense,
    Steal,
    Attack,
}

#[derive(Debug, Clone)]
struct SimulationSetting {
    m: usize,
    start_turn: usize,
    end_turn: usize,
    params: Vec<PlayerParam>,
    is_random: Vec<bool>,
    random_r: Vec<f32>,
}

impl SimulationSetting {
    fn new(
        m: usize,
        start_turn: usize,
        end_turn: usize,
        params: Vec<PlayerParam>,
        rng: &mut XorShift32,
    ) -> Self {
        let mut is_random = vec![false; (end_turn - start_turn) * m];
        for turn in 0..(end_turn - start_turn) {
            for p in 1..m {
                is_random[turn * m + p] = rng.gen_f32() < params[p].eps;
            }
        }
        let random_r = (0..(end_turn - start_turn) * m)
            .map(|_| rng.gen_f32())
            .collect_vec();
        Self {
            m,
            start_turn,
            end_turn,
            params,
            is_random,
            random_r,
        }
    }

    fn get_is_random(&self, p: usize, turn: usize) -> bool {
        debug_assert!(1 <= p && p < self.m);
        debug_assert!(self.start_turn <= turn && turn < self.end_turn);
        self.is_random[(turn - self.start_turn) * self.m + p]
    }

    fn get_random_r(&self, p: usize, turn: usize) -> f32 {
        debug_assert!(1 <= p && p < self.m);
        debug_assert!(self.start_turn <= turn && turn < self.end_turn);
        self.random_r[(turn - self.start_turn) * self.m + p]
    }

    fn num_simulation_turns(&self) -> usize {
        self.end_turn - self.start_turn
    }
}

#[derive(Debug, Clone, Default)]
struct AiPolicy {
    actions: u128,
    evals: [f32; 4],
    bests: [(usize, usize); 4],
    prob: [f32; 4],
    other_prob: f32,
}

impl AiPolicy {
    fn new(actions: u128, evals: [f32; 4], bests: [(usize, usize); 4]) -> Self {
        Self {
            actions,
            evals,
            bests,
            prob: [0.0; 4],
            other_prob: 0.0,
        }
    }

    fn get_greedy_action(&self, param: &PlayerParam) -> (usize, usize) {
        let mut best_i = 0;
        let mut max_profit = f32::MIN;
        for i in 0..4 {
            let profit = self.evals[i] * param.w[i];
            if max_profit.chmax(profit) {
                best_i = i;
            }
        }
        self.bests[best_i]
    }

    fn update_prob(&mut self, params: &[PlayerParam]) {
        self.prob.fill(0.0);
        let mut eps = 0.0;
        for param in params {
            let mut best_i = 0;
            let mut max_profit = f32::MIN;
            for i in 0..4 {
                let profit = self.evals[i] * param.w[i];
                if max_profit.chmax(profit) {
                    best_i = i;
                }
            }
            self.prob[best_i] += 1.0 - param.eps;
            eps += param.eps;
        }
        let num_samples = params.len() as f32;
        for i in 0..4 {
            self.prob[i] /= num_samples;
        }
        eps /= num_samples;
        self.other_prob = eps / self.actions.count_ones() as f32;
    }

    fn chosen_prob(&self, x: usize, y: usize) -> f32 {
        if !get128(self.actions, x * N1 + y) {
            return 0.0;
        }
        let mut prob = self.other_prob;
        for i in 0..4 {
            if self.bests[i] == (x, y) {
                prob += self.prob[i];
                break;
            }
        }
        prob
    }
}

/// (1 - eps) occurs and a[i] * w[i] < w[chosen] for all i
/// or
/// eps / n occurs
#[derive(Debug, Clone)]
struct GreedyResults {
    a: [f32; 4],
    chosen: usize,
    n: f32,
    log_n: f32,
}

impl GreedyResults {
    fn new(mut evals: [f32; 4], chosen: usize, n: usize) -> Self {
        let eval_chosen = evals[chosen];
        for i in 0..4 {
            evals[i] /= eval_chosen;
        }
        Self {
            a: evals,
            chosen,
            n: n as f32,
            log_n: (n as f32).log2(),
        }
    }

    fn is_satisfied(&self, w: &[f32]) -> bool {
        let w_chosen = w[self.chosen];
        for i in 0..4 {
            if i == self.chosen {
                continue;
            }
            if w[i] * self.a[i] > w_chosen {
                return false;
            }
        }
        true
    }

    fn log_likelihood(&self, param: &PlayerParam, log_eps: f32) -> f32 {
        if self.is_satisfied(&param.w) {
            log2_fast((1.0 - param.eps) + param.eps / self.n)
        } else {
            log_eps - self.log_n
        }
    }
}

#[derive(Debug, Clone)]
struct RandomResults {
    num_res: f32,
    sum_log: f32,
}

impl RandomResults {
    fn new() -> Self {
        Self {
            num_res: 0.0,
            sum_log: 0.0,
        }
    }

    fn add_result(&mut self, num_actions: usize) {
        debug_assert!(num_actions >= 2);
        self.num_res += 1.0;
        self.sum_log += ((num_actions - 1) as f32 / num_actions as f32).log2();
    }

    fn log_likelihood(&self, log_eps: f32) -> f32 {
        self.num_res * log_eps - self.sum_log
    }
}

#[derive(Debug, Clone)]
struct Environment {
    m: usize,
    lv_max: i32,
    values: [[f32; N]; N],
    connected3x3: FixedBitSet,
    initial_pos: Vec<(usize, usize)>,
    top_pos: (usize, usize),
    rng: XorShift32,
    greedy_results: Vec<Vec<GreedyResults>>,
    random_results: Vec<RandomResults>,
    params: Vec<Vec<PlayerParam>>,
    sched: AnnealingScheduler,
    w_delta: f32,
}

impl Environment {
    fn new(input: &Input) -> Self {
        let mut rng = XorShift32::new(1);
        let mut params = vec![Vec::with_capacity(NUM_SAMPLES_FIRST); input.m];
        let mut w = vec![];
        for _ in 0..4 {
            let w_delta = (W_MAX - W_MIN) / NUM_SAMPLES_FIRST as f32;
            w.push(
                (0..NUM_SAMPLES_FIRST)
                    .map(|i| W_MIN + i as f32 * w_delta + w_delta / 2.0)
                    .collect_vec(),
            );
            rng.shuffle(&mut w.last_mut().unwrap());
        }
        let mut w = (0..NUM_SAMPLES_FIRST)
            .map(|i| [w[0][i], w[1][i], w[2][i], w[3][i]])
            .collect_vec();

        for p in 0..input.m {
            rng.shuffle(&mut w);
            for i in 0..NUM_SAMPLES_FIRST {
                let eps = EPS_MIN; // rng.gen_range_f32(EPS_MIN, EPS_MAX);
                params[p].push(PlayerParam::new(w[i], eps));
            }
        }
        let mut res = Self {
            m: input.m,
            lv_max: input.lv_max,
            values: input.values,
            connected3x3: make_connected3x3(),
            initial_pos: input.initial_pos.clone(),
            top_pos: (!0, !0),
            rng: XorShift32::new(1),
            greedy_results: vec![vec![]; input.m],
            random_results: vec![RandomResults::new(); input.m],
            params,
            sched: AnnealingScheduler::new(SCHED_TYPE, T0, T1, TIME_LIMIT_SEC),
            w_delta: 0.0,
        };
        res.set_top();
        res
    }

    fn set_top(&mut self) {
        let mut max_value = f32::MIN;
        for x in 0..N {
            for y in 0..N {
                let dist = distance_from_center(x, y);
                let value = self.values[x][y] + TOP_DIST_WEIGHT * dist;
                if max_value.chmax(value) {
                    self.top_pos = (x, y);
                }
            }
        }
    }

    fn distance_from_top(&self, x: usize, y: usize) -> f32 {
        let (tx, ty) = self.top_pos;
        let dx = tx as f32 - x as f32;
        let dy = ty as f32 - y as f32;
        dx.abs() + dy.abs()
    }

    fn add_greedy_result(&mut self, p: usize, res: GreedyResults) {
        for param in &mut self.params[p] {
            param.log_likelihood += res.log_likelihood(param, param.eps.log2());
        }
        self.greedy_results[p].push(res);
    }

    fn add_random_result(&mut self, p: usize, num_actions: usize) {
        for param in &mut self.params[p] {
            param.log_likelihood -= self.random_results[p].log_likelihood(param.eps.log2());
        }
        self.random_results[p].add_result(num_actions);
        for param in &mut self.params[p] {
            param.log_likelihood += self.random_results[p].log_likelihood(param.eps.log2());
        }
    }

    fn set_log_likelihood(&self, p: usize, param: &mut PlayerParam) {
        let log_eps = param.eps.log2();
        let mut log_likelihood = self.random_results[p].log_likelihood(log_eps);
        for res in &self.greedy_results[p] {
            log_likelihood += res.log_likelihood(param, log_eps);
        }
        param.log_likelihood = log_likelihood;
    }

    fn sample_params(&mut self, turn: usize) {
        let start_time = get_time_sec();
        if start_time > TIME_LIMIT_SEC {
            return;
        }
        let progress = turn as f32 / T as f32;
        self.w_delta = 0.5 * (1.0 - progress) + 0.2 * progress;

        let num_samples = (NUM_SAMPLES_FIRST as f32 * (1.0 - progress)
            + NUM_SAMPLES_LAST as f32 * progress) as usize;
        for p in 0..self.m {
            self.params[p].truncate(num_samples);
        }

        for p in 1..self.m {
            let end_time = start_time + p as f64 * MCMC_TIME_PER_TURN_SEC / (self.m - 1) as f64;
            while get_time_sec() < end_time {
                for i in 0..self.params[p].len() {
                    let mut param = self.resample(self.params[p][i].clone());
                    self.set_log_likelihood(p, &mut param);
                    let profit = param.log_likelihood - self.params[p][i].log_likelihood;
                    if self.sched.accept(profit as f64) {
                        self.params[p][i] = param;
                    }
                }
            }
        }
    }

    fn resample(&mut self, mut param: PlayerParam) -> PlayerParam {
        let r = self.rng.gen_range(0, R_RESAMPLE_EPS);
        if r < R_RESAMPLE_W {
            self.update_w(&mut param);
        } else {
            self.update_eps(&mut param);
        }
        param
    }

    fn update_w(&mut self, param: &mut PlayerParam) {
        if self.rng.gen_range(0, RESAMPLE_W_INV) == 0 {
            for i in 0..4 {
                param.w[i] = self.rng.gen_range_f32(W_MIN, W_MAX);
            }
            return;
        }
        if self.lv_max == 1 {
            if self.rng.gen_range(0, 2) == 0 {
                param.w[0] = self.rng.gen_range_f32(W_MIN, W_MAX);
            } else {
                param.w[2] = self.rng.gen_range_f32(W_MIN, W_MAX);
            }
        } else {
            let i = self.rng.gen_range(0, 4);
            param.w[i] = self.rng.gen_range_f32(W_MIN, W_MAX);

            // random rescaling
            let mut w_min = f32::MAX;
            let mut w_max = f32::MIN;
            for j in 0..4 {
                if j == i {
                    continue;
                }
                w_min.chmin(param.w[j]);
                w_max.chmax(param.w[j]);
            }
            let r_min = W_MIN / w_min;
            let r_max = W_MAX / w_max;
            let r = self.rng.gen_range_f32(r_min, r_max);
            for j in 0..4 {
                if j == i {
                    continue;
                }
                param.w[j] *= r;
            }
        }
    }

    fn update_eps(&mut self, param: &mut PlayerParam) {
        param.eps = self.rng.gen_range_f32(EPS_MIN, EPS_MAX);
    }
}

#[cfg(feature = "cheat")]
fn read_player_params(m: usize) -> Vec<PlayerParam> {
    let mut params = vec![PlayerParam::default(); m];
    for p in 1..m {
        input_interactive! {
            w: [f32; 4],
            eps: f32,
        }
        params[p] = PlayerParam {
            w: w.try_into().unwrap(),
            eps,
            log_likelihood: 0.0,
        };
    }
    params
}

#[derive(Debug, Clone, Default)]
struct PlayerParam {
    w: [f32; 4],
    eps: f32,
    log_likelihood: f32,
}

impl PlayerParam {
    fn new(w: [f32; 4], eps: f32) -> Self {
        Self {
            w,
            eps,
            log_likelihood: 0.0,
        }
    }

    #[allow(dead_code)]
    fn average() -> Self {
        Self {
            w: [(W_MIN + W_MAX) / 2.0; 4],
            eps: (EPS_MIN + EPS_MAX) / 2.0,
            log_likelihood: 0.0,
        }
    }

    #[allow(dead_code)]
    fn sample(rng: &mut XorShift32) -> Self {
        let mut w = [0.0; 4];
        for i in 0..4 {
            w[i] = rng.gen_range_f32(W_MIN, W_MAX);
        }
        let eps = rng.gen_range_f32(EPS_MIN, EPS_MAX);
        Self {
            w,
            eps,
            log_likelihood: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Input {
    m: usize,
    lv_max: i32,
    values: [[f32; N]; N],
    initial_pos: Vec<(usize, usize)>,
}

fn read_input() -> Input {
    input_interactive! {
        _n: usize,
        m: usize,
        _t: usize,
        lv_max: i32,
        values: [[f32; N]; N],
        initial_pos: [(usize, usize); m],
    }
    assert_eq!(_n, N);
    assert!(2 <= m && m <= 8);
    assert_eq!(_t, T);
    assert!(1 <= lv_max && lv_max <= 5);
    Input {
        m,
        lv_max,
        values: values
            .into_iter()
            .map(|row| row.try_into().unwrap())
            .collect_vec()
            .try_into()
            .unwrap(),
        initial_pos,
    }
}

fn bfs128(sx: usize, sy: usize, valid: u128) -> u128 {
    let mut res = 1 << (sx * N1 + sy);
    loop {
        let next = expand128(res) & valid;
        if res == next {
            return res;
        }
        res = next;
    }
}

#[inline(always)]
fn expand128(b: u128) -> u128 {
    let mut res = b;
    res |= b >> N1;
    res |= b << N1;
    res |= b >> 1;
    res |= b << 1;
    res & IN_GRID
}

#[inline(always)]
fn get128(b: u128, pos: usize) -> bool {
    (b & (1 << pos)) != 0
}

#[inline(always)]
fn set128(b: &mut u128, pos: usize, enabled: bool) {
    if enabled {
        *b |= 1 << pos;
    } else {
        *b &= !(1 << pos);
    }
}

#[inline(always)]
fn pop128(b: &mut u128) -> usize {
    debug_assert!(*b != 0);
    let pos = b.trailing_zeros() as usize;
    *b ^= 1 << pos;
    pos
}

#[inline(always)]
fn select128(b: u128, i: usize) -> usize {
    debug_assert!(i < b.count_ones() as usize);
    let low64 = b as u64;
    let low64_cnt = low64.count_ones() as usize;
    if i < low64_cnt {
        select64(low64, i)
    } else {
        let high64 = (b >> 64) as u64;
        select64(high64, i - low64_cnt) + 64
    }
}

#[inline(always)]
fn select64(b: u64, i: usize) -> usize {
    // TODO: use _pdep_u64 when targeting x86_64
    debug_assert!(i < b.count_ones() as usize);
    let low32 = b as u32;
    let low32_cnt = low32.count_ones() as usize;
    if i < low32_cnt {
        select32(low32, i)
    } else {
        let high32 = (b >> 32) as u32;
        select32(high32, i - low32_cnt) + 32
    }
}

#[inline(always)]
fn select32(mut b: u32, i: usize) -> usize {
    debug_assert!(i < b.count_ones() as usize);
    for _ in 0..i {
        b &= b - 1;
    }
    b.trailing_zeros() as usize
}

#[inline(always)]
fn distance_from_center(x: usize, y: usize) -> f32 {
    let dx = x as f32 - CENTER;
    let dy = y as f32 - CENTER;
    dx.abs() + dy.abs()
}

#[inline(always)]
pub fn log2_fast(x: f32) -> f32 {
    let y = (x.to_bits() as i32) - (127 << 23);
    y as f32 / (1 << 23) as f32
}

// This function aids in making a conservative assessment of
// whether a vertex within a grid is an articulation point,
// considering only the eight surrounding vertices.
// 0 1 2
// 3 4 5
// 6 7 8
pub fn make_connected3x3() -> FixedBitSet {
    let mut edges = vec![Vec::new(); 9];
    for v in 0..9 {
        if v / 3 > 0 {
            edges[v].push(v - 3);
        }
        if v % 3 > 0 {
            edges[v].push(v - 1);
        }
        if v % 3 < 2 {
            edges[v].push(v + 1);
        }
        if v / 3 < 2 {
            edges[v].push(v + 3);
        }
    }
    let mut connected3x3 = FixedBitSet::with_capacity(512);
    connected3x3.set(0, true);
    for s in 1..512 as usize {
        // Depth First Search
        let root = s.trailing_zeros();
        let mut visited = 1 << root;
        let mut todo: usize = 1 << root;
        while todo > 0 {
            let u = todo.trailing_zeros() as usize;
            todo ^= 1 << u;
            for &v in &edges[u] {
                if (s & (1 << v)) > 0 && (visited & (1 << v)) == 0 {
                    visited |= 1 << v;
                    todo |= 1 << v;
                }
            }
        }
        if visited == s {
            connected3x3.set(s, true);
        }
    }
    connected3x3
}

const SA_TIME_COUNTS: usize = 1 << 12;
const SA_RANDOM_STEPS: usize = 1 << 10;

#[derive(Debug, Clone)]
pub enum SchedulerType {
    Exp,
    Linear,
}

#[derive(Debug, Clone)]
pub struct AnnealingScheduler {
    schedule_type: SchedulerType,
    t_first: f64,
    t_last: f64,
    start_time_sec: f64,
    duration_sec: f64,
    time_counter: usize,
    temperature: f64,
    random_index: usize,
    log2_random: Vec<f64>,
    trials: usize,
    acceptances: usize,
    progress: f64,
}

impl AnnealingScheduler {
    pub fn new(
        schedule_type: SchedulerType,
        t_first: f64,
        t_last: f64,
        time_limit_sec: f64,
    ) -> Self {
        debug_assert!(0.0 <= t_last && t_last <= t_first);

        let mut log2_random = vec![0.0; SA_RANDOM_STEPS];
        for i in 0..SA_RANDOM_STEPS {
            log2_random[i] = ((i + 1) as f64 / SA_RANDOM_STEPS as f64).log2();
        }

        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        let mut rng = rand_pcg::Pcg64Mcg::seed_from_u64(0);
        log2_random.shuffle(&mut rng);
        Self {
            schedule_type,
            t_first,
            t_last,
            start_time_sec: get_time_sec(),
            duration_sec: time_limit_sec - get_time_sec(),
            time_counter: 0,
            temperature: t_first,
            random_index: 0,
            log2_random,
            trials: 0,
            acceptances: 0,
            progress: 0.0,
        }
    }

    pub fn accept(&mut self, profit: f64) -> bool {
        if profit >= 0.0 || profit > self.get_threshold() {
            self.accepted();
            return true;
        } else {
            self.rejected();
            return false;
        }
    }

    pub fn accepted(&mut self) {
        self.trials += 1;
        self.acceptances += 1;
    }

    pub fn rejected(&mut self) {
        self.trials += 1;
    }

    pub fn get_threshold(&mut self) -> f64 {
        self.update_temperature();
        if self.random_index == SA_RANDOM_STEPS - 1 {
            self.random_index = 0;
        } else {
            self.random_index += 1;
        }
        self.temperature * self.log2_random[self.random_index]
    }

    fn update_temperature(&mut self) {
        if self.time_counter > 0 {
            self.time_counter -= 1;
            return;
        }
        self.time_counter = SA_TIME_COUNTS - 1;
        self.progress = (get_time_sec() - self.start_time_sec) / self.duration_sec;
        self.temperature = match self.schedule_type {
            SchedulerType::Exp => {
                self.t_first.powf(1.0 - self.progress) * self.t_last.powf(self.progress)
            }
            SchedulerType::Linear => {
                self.t_first * (1.0 - self.progress) + self.t_last * self.progress
            }
        }
    }

    pub fn print_log(&self) {
        let acceptance_rate = self.acceptances as f64 / self.trials as f64;
        eprintln!("trial : {}", self.trials);
        eprintln!("accept: {}", self.acceptances);
        eprintln!("rate  : {}", acceptance_rate);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    pub fn new(mut seed: u32) -> Self {
        if seed == 0 {
            seed = u32::MAX;
        }
        Self { state: seed }
    }

    /// 1..=u32::MAX
    #[inline(always)]
    pub fn gen_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// 0.0..=1.0
    #[inline(always)]
    pub fn gen_f32(&mut self) -> f32 {
        self.gen_u32() as f32 / ((1u64 << 32) as f32)
    }

    /// 0.0..=1.0
    #[inline(always)]
    pub fn gen_f64(&mut self) -> f64 {
        self.gen_u32() as f64 / ((1u64 << 32) as f64)
    }

    /// l..r
    #[inline(always)]
    pub fn gen_range(&mut self, l: usize, r: usize) -> usize {
        debug_assert!(l < r);
        debug_assert!(r as u64 <= 1 << 32);
        l + (((r - l) as u64 * self.gen_u32() as u64) >> 32) as usize
    }

    /// l..=r
    #[inline(always)]
    pub fn gen_range_f32(&mut self, l: f32, r: f32) -> f32 {
        debug_assert!(l <= r);
        l + self.gen_u32() as f32 * ((r - l) / ((1u64 << 32) as f32))
    }

    /// l..=r
    #[inline(always)]
    pub fn gen_range_f64(&mut self, l: f64, r: f64) -> f64 {
        debug_assert!(l <= r);
        l + self.gen_u32() as f64 * ((r - l) / ((1u64 << 32) as f64))
    }

    #[inline(always)]
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        let n = v.len();
        for i in (1..n).rev() {
            let j = self.gen_range(0, i + 1);
            v.swap(i, j);
        }
    }

    #[inline(always)]
    pub fn partial_shuffle<T>(&mut self, v: &mut [T], n: usize) {
        let m = v.len();
        debug_assert!(n <= m);
        for i in 0..n {
            let j = self.gen_range(i, m);
            v.swap(i, j);
        }
    }
}

pub trait ChangeMinMax {
    fn chmin(&mut self, x: Self) -> bool;
    fn chmax(&mut self, x: Self) -> bool;
}

impl<T: PartialOrd> ChangeMinMax for T {
    fn chmin(&mut self, x: T) -> bool {
        *self > x && {
            *self = x;
            true
        }
    }

    fn chmax(&mut self, x: T) -> bool {
        *self < x && {
            *self = x;
            true
        }
    }
}

pub fn get_time_sec() -> f64 {
    static mut STIME: f64 = -1.0;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let ms = t.as_secs() as f64 + t.subsec_nanos() as f64 * 1e-9;
    unsafe {
        if STIME < 0.0 {
            STIME = ms;
        }
        ms - STIME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_in_grid() {
        let mut in_grid = 0u128;
        for x in 0..10 {
            for y in 0..10 {
                in_grid |= 1 << (x * N1 + y);
            }
        }
        println!("{:#x}", in_grid);
    }
}
