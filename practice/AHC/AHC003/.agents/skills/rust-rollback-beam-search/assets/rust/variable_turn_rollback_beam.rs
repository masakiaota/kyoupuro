#![allow(dead_code)]

use std::cmp::Ordering;
use std::collections::HashMap;

pub type HashKey = u64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Action {
    pub id: u8,
    pub step: usize,
    pub score_delta: i64,
    pub hash_delta: u64,
}

impl Action {
    // TODO(problem): replace this with a problem-specific compact action.
    fn sample(id: u8, step: usize) -> Self {
        Self {
            id,
            step,
            score_delta: 10 * step as i64 - id as i64,
            hash_delta: 0xA076_1D64_78BD_642F_u64.wrapping_mul(id as u64 + step as u64 + 1),
        }
    }

    pub fn step(self) -> usize {
        self.step
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Evaluator {
    // Smaller is better. For score maximization, store `-score`.
    pub score_key: i64,
    pub tie_break: u32,
}

#[derive(Debug)]
pub struct State {
    // TODO(problem): replace this example state with the real mutable state.
    turn: usize,
    score: i64,
    hash: u64,
}

impl State {
    pub fn new() -> Self {
        Self {
            turn: 0,
            score: 0,
            hash: 0,
        }
    }

    pub fn enumerate_actions(&self, _turn: usize) -> Vec<Action> {
        // TODO(problem): generate legal macro actions. Each action must have step >= 1.
        vec![Action::sample(0, 1), Action::sample(1, 2)]
    }

    pub fn move_forward(&mut self, action: Action) {
        // TODO(problem): apply action and update all incremental fields.
        assert!(action.step() > 0);
        self.turn += action.step();
        self.score += action.score_delta;
        self.hash ^= action.hash_delta;
    }

    pub fn move_backward(&mut self, action: Action) {
        // TODO(problem): undo exactly what move_forward did.
        self.hash ^= action.hash_delta;
        self.score -= action.score_delta;
        self.turn -= action.step();
    }

    pub fn evaluate(&self, _turn: usize) -> Evaluator {
        // TODO(problem): tune the beam ordering. Smaller is better.
        Evaluator {
            score_key: -self.score,
            tie_break: self.turn as u32,
        }
    }

    pub fn hash_key(&self) -> HashKey {
        // TODO(problem): include all fields needed for safe deduplication.
        self.hash
    }

    pub fn is_terminal(&self) -> bool {
        // TODO(problem): return true if this state already represents a complete answer.
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SlotValue {
    evaluator: Evaluator,
    index: usize,
}

impl Ord for SlotValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.evaluator
            .cmp(&other.evaluator)
            .then_with(|| self.index.cmp(&other.index))
    }
}

impl PartialOrd for SlotValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct MaxSegTree {
    size: usize,
    data: Vec<Option<SlotValue>>,
}

impl MaxSegTree {
    fn new(len: usize) -> Self {
        let size = len.max(1).next_power_of_two();
        Self {
            size,
            data: vec![None; size * 2],
        }
    }

    fn set(&mut self, mut index: usize, value: Option<SlotValue>) {
        index += self.size;
        self.data[index] = value;
        while index > 1 {
            index >>= 1;
            self.data[index] = self.data[index * 2].max(self.data[index * 2 + 1]);
        }
    }

    fn max_all(&self) -> Option<SlotValue> {
        self.data[1]
    }
}

#[derive(Clone, Copy, Debug)]
struct BeamCandidate {
    parent: usize,
    action: Action,
    evaluator: Evaluator,
    hash_key: HashKey,
    turn: usize,
}

#[derive(Debug)]
struct Selector {
    capacity: usize,
    candidates: Vec<Option<BeamCandidate>>,
    by_hash: HashMap<HashKey, usize>,
    worst: MaxSegTree,
}

impl Selector {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            candidates: Vec::with_capacity(capacity),
            by_hash: HashMap::with_capacity(capacity * 2 + 1),
            worst: MaxSegTree::new(capacity),
        }
    }

    fn clear(&mut self) {
        self.candidates.clear();
        self.by_hash.clear();
        self.worst = MaxSegTree::new(self.capacity);
    }

    fn push(&mut self, candidate: BeamCandidate) {
        if self.capacity == 0 {
            return;
        }

        if let Some(&index) = self.by_hash.get(&candidate.hash_key) {
            let current = self.candidates[index].expect("selector slot must be occupied");
            if candidate.evaluator < current.evaluator {
                self.candidates[index] = Some(candidate);
                self.worst.set(
                    index,
                    Some(SlotValue {
                        evaluator: candidate.evaluator,
                        index,
                    }),
                );
            }
            return;
        }

        if self.candidates.len() < self.capacity {
            let index = self.candidates.len();
            self.candidates.push(Some(candidate));
            self.by_hash.insert(candidate.hash_key, index);
            self.worst.set(
                index,
                Some(SlotValue {
                    evaluator: candidate.evaluator,
                    index,
                }),
            );
            return;
        }

        let Some(worst) = self.worst.max_all() else {
            return;
        };
        if candidate.evaluator >= worst.evaluator {
            return;
        }

        let old = self.candidates[worst.index].expect("selector slot must be occupied");
        self.by_hash.remove(&old.hash_key);
        self.candidates[worst.index] = Some(candidate);
        self.by_hash.insert(candidate.hash_key, worst.index);
        self.worst.set(
            worst.index,
            Some(SlotValue {
                evaluator: candidate.evaluator,
                index: worst.index,
            }),
        );
    }

    fn take_sorted(&mut self) -> Vec<BeamCandidate> {
        let mut result = self
            .candidates
            .iter()
            .filter_map(|candidate| *candidate)
            .collect::<Vec<_>>();
        result.sort_unstable_by(|a, b| {
            a.evaluator
                .cmp(&b.evaluator)
                .then_with(|| a.hash_key.cmp(&b.hash_key))
        });
        self.clear();
        result
    }
}

#[derive(Clone, Debug)]
struct BeamNode {
    parent: usize,
    action: Option<Action>,
    evaluator: Evaluator,
    hash_key: HashKey,
    turn: usize,
}

#[derive(Clone, Copy, Debug)]
enum TourEdge {
    Forward(usize),
    Backward(usize),
    Visit(usize),
}

fn path_from_root(nodes: &[BeamNode], mut node: usize) -> Vec<usize> {
    let mut path = Vec::new();
    while node != 0 {
        path.push(node);
        node = nodes[node].parent;
    }
    path.reverse();
    path
}

fn build_tour_edges(leaves: &[usize], nodes: &[BeamNode]) -> Vec<TourEdge> {
    let mut edges = Vec::new();
    let mut previous_path = Vec::<usize>::new();

    for &leaf in leaves {
        let path = path_from_root(nodes, leaf);
        let mut lcp = 0;
        while lcp < previous_path.len() && lcp < path.len() && previous_path[lcp] == path[lcp] {
            lcp += 1;
        }

        for &node in previous_path[lcp..].iter().rev() {
            edges.push(TourEdge::Backward(node));
        }
        for &node in &path[lcp..] {
            edges.push(TourEdge::Forward(node));
        }
        edges.push(TourEdge::Visit(leaf));
        previous_path = path;
    }

    for &node in previous_path.iter().rev() {
        edges.push(TourEdge::Backward(node));
    }

    edges
}

pub fn rollback_beam_search(mut state: State, max_turn: usize, beam_width: usize) -> Vec<Action> {
    let root = BeamNode {
        parent: 0,
        action: None,
        evaluator: state.evaluate(0),
        hash_key: state.hash_key(),
        turn: 0,
    };
    let mut nodes = vec![root];
    let mut beams_by_turn = vec![Vec::<usize>::new(); max_turn + 1];
    beams_by_turn[0].push(0);
    let mut selectors = (0..=max_turn)
        .map(|_| Selector::new(beam_width))
        .collect::<Vec<_>>();
    let mut best_leaf = None::<usize>;

    for turn in 0..=max_turn {
        let selected = selectors[turn].take_sorted();
        for candidate in selected {
            let node = BeamNode {
                parent: candidate.parent,
                action: Some(candidate.action),
                evaluator: candidate.evaluator,
                hash_key: candidate.hash_key,
                turn: candidate.turn,
            };
            nodes.push(node);
            beams_by_turn[turn].push(nodes.len() - 1);
        }

        if beams_by_turn[turn].is_empty() {
            continue;
        }

        if let Some(local_best) = beams_by_turn[turn]
            .iter()
            .copied()
            .min_by(|&a, &b| nodes[a].evaluator.cmp(&nodes[b].evaluator))
        {
            if best_leaf
                .map(|best| nodes[local_best].evaluator < nodes[best].evaluator)
                .unwrap_or(true)
            {
                best_leaf = Some(local_best);
            }
        }

        let tour_edges = build_tour_edges(&beams_by_turn[turn], &nodes);
        for edge in tour_edges {
            match edge {
                TourEdge::Forward(node) => {
                    state.move_forward(nodes[node].action.expect("root is never forwarded"));
                }
                TourEdge::Backward(node) => {
                    state.move_backward(nodes[node].action.expect("root is never rolled back"));
                }
                TourEdge::Visit(parent) => {
                    debug_assert_eq!(nodes[parent].turn, turn);
                    if state.is_terminal() {
                        continue;
                    }
                    for action in state.enumerate_actions(turn) {
                        let step = action.step();
                        assert!(step > 0, "variable-turn actions must advance at least one turn");
                        let next_turn = turn + step;
                        if next_turn > max_turn {
                            continue;
                        }

                        state.move_forward(action);
                        selectors[next_turn].push(BeamCandidate {
                            parent,
                            action,
                            evaluator: state.evaluate(next_turn),
                            hash_key: state.hash_key(),
                            turn: next_turn,
                        });
                        state.move_backward(action);
                    }
                }
            }
        }
    }

    reconstruct_actions(&nodes, best_leaf.unwrap_or(0))
}

fn reconstruct_actions(nodes: &[BeamNode], mut node: usize) -> Vec<Action> {
    let mut actions = Vec::new();
    while node != 0 {
        actions.push(nodes[node].action.expect("non-root node must have an action"));
        node = nodes[node].parent;
    }
    actions.reverse();
    actions
}

fn main() {
    // This file is a standalone-compilable template.
    let _actions = rollback_beam_search(State::new(), 5, 4);
}
