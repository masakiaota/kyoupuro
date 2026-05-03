#![allow(dead_code)]

use std::cmp::Ordering;
use std::collections::HashMap;

pub type HashKey = u64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Action {
    pub id: u8,
    pub score_delta: i64,
    pub hash_delta: u64,
}

impl Action {
    // TODO(problem): replace this with a problem-specific compact action.
    fn sample(id: u8) -> Self {
        Self {
            id,
            score_delta: 10 - id as i64,
            hash_delta: 0xD1B5_4A32_D192_ED03_u64.wrapping_mul(id as u64 + 1),
        }
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
        // TODO(problem): generate only promising legal actions from the current state.
        vec![Action::sample(0), Action::sample(1)]
    }

    pub fn move_forward(&mut self, action: Action) {
        // TODO(problem): apply action and update all incremental fields.
        self.turn += 1;
        self.score += action.score_delta;
        self.hash ^= action.hash_delta;
    }

    pub fn move_backward(&mut self, action: Action) {
        // TODO(problem): undo exactly what move_forward did.
        self.hash ^= action.hash_delta;
        self.score -= action.score_delta;
        self.turn -= 1;
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
    first_child: Option<usize>,
    next_sibling: Option<usize>,
    action: Option<Action>,
    evaluator: Evaluator,
    hash_key: HashKey,
    depth: usize,
}

fn attach_child(nodes: &mut [BeamNode], parent: usize, child: usize) {
    nodes[child].next_sibling = nodes[parent].first_child;
    nodes[parent].first_child = Some(child);
}

fn expand_current_depth(
    node: usize,
    target_depth: usize,
    state: &mut State,
    nodes: &[BeamNode],
    selector: &mut Selector,
) {
    let depth = nodes[node].depth;
    if depth == target_depth {
        for action in state.enumerate_actions(target_depth) {
            state.move_forward(action);
            selector.push(BeamCandidate {
                parent: node,
                action,
                evaluator: state.evaluate(target_depth + 1),
                hash_key: state.hash_key(),
            });
            state.move_backward(action);
        }
        return;
    }

    let mut child = nodes[node].first_child;
    while let Some(child_node) = child {
        let action = nodes[child_node]
            .action
            .expect("non-root node must have an action");
        state.move_forward(action);
        expand_current_depth(child_node, target_depth, state, nodes, selector);
        state.move_backward(action);
        child = nodes[child_node].next_sibling;
    }
}

pub fn rollback_beam_search(mut state: State, max_turn: usize, beam_width: usize) -> Vec<Action> {
    let root = BeamNode {
        parent: 0,
        first_child: None,
        next_sibling: None,
        action: None,
        evaluator: state.evaluate(0),
        hash_key: state.hash_key(),
        depth: 0,
    };
    let mut nodes = vec![root];
    let mut beam = vec![0_usize];
    let mut selector = Selector::new(beam_width);

    for turn in 0..max_turn {
        selector.clear();
        expand_current_depth(0, turn, &mut state, &nodes, &mut selector);

        let selected = selector.take_sorted();
        if selected.is_empty() {
            break;
        }

        beam.clear();
        for candidate in selected {
            let parent_depth = nodes[candidate.parent].depth;
            let node = BeamNode {
                parent: candidate.parent,
                first_child: None,
                next_sibling: None,
                action: Some(candidate.action),
                evaluator: candidate.evaluator,
                hash_key: candidate.hash_key,
                depth: parent_depth + 1,
            };
            nodes.push(node);
            let child = nodes.len() - 1;
            attach_child(&mut nodes, candidate.parent, child);
            beam.push(child);
        }
    }

    let best_leaf = beam
        .iter()
        .copied()
        .min_by(|&a, &b| nodes[a].evaluator.cmp(&nodes[b].evaluator))
        .unwrap_or(0);
    reconstruct_actions(&nodes, best_leaf)
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
    let _actions = rollback_beam_search(State::new(), 3, 4);
}
