// euler_tour_edges_rollback_beam.cpp
#include <bits/stdc++.h>

using namespace std;

using HashKey = uint64_t;

struct Action {
    uint8_t id = 0;
    int64_t score_delta = 0;
    uint64_t hash_delta = 0;

    // TODO(problem): 問題固有の小さな Action に置き換える。
    static Action sample(uint8_t id) {
        return {
            id,
            10 - static_cast<int64_t>(id),
            0x9E3779B97F4A7C15ULL * (static_cast<uint64_t>(id) + 1),
        };
    }
};

struct Evaluator {
    // 小さいほど良い。スコア最大化なら -score を格納する。
    int64_t score_key = 0;
    uint32_t tie_break = 0;

    auto operator<=>(const Evaluator&) const = default;
};

class State {
public:
    // TODO(problem): 実際の差分更新可能な状態に置き換える。
    vector<Action> enumerate_actions([[maybe_unused]] size_t turn) const {
        return {Action::sample(0), Action::sample(1)};
    }

    void move_forward(const Action& action) {
        // TODO(problem): Action を適用し、すべての差分管理値を更新する。
        ++turn_;
        score_ += action.score_delta;
        hash_ ^= action.hash_delta;
    }

    void move_backward(const Action& action) {
        // TODO(problem): move_forward と逆順に、状態を完全に戻す。
        hash_ ^= action.hash_delta;
        score_ -= action.score_delta;
        --turn_;
    }

    Evaluator evaluate([[maybe_unused]] size_t turn) const {
        // TODO(problem): beam 内の順序を設計する。
        return {-score_, static_cast<uint32_t>(turn_)};
    }

    HashKey hash_key() const {
        // TODO(problem): 安全な重複排除に必要な状態を含める。
        return hash_;
    }

private:
    size_t turn_ = 0;
    int64_t score_ = 0;
    uint64_t hash_ = 0;
};

struct SlotValue {
    Evaluator evaluator;
    size_t index = 0;

    auto operator<=>(const SlotValue&) const = default;
};

class MaxSegTree {
public:
    explicit MaxSegTree(size_t length) {
        while (size_ < max<size_t>(1, length)) {
            size_ <<= 1;
        }
        data_.resize(size_ * 2);
    }

    void set(size_t index, optional<SlotValue> value) {
        index += size_;
        data_[index] = value;
        while (index > 1) {
            index >>= 1;
            data_[index] = max(data_[index * 2], data_[index * 2 + 1]);
        }
    }

    optional<SlotValue> max_all() const {
        return data_[1];
    }

private:
    size_t size_ = 1;
    vector<optional<SlotValue>> data_;
};

struct BeamCandidate {
    size_t parent = 0;
    Action action;
    Evaluator evaluator;
    HashKey hash_key = 0;
};

class Selector {
public:
    explicit Selector(size_t capacity)
        : capacity_(capacity), worst_(capacity) {
        candidates_.reserve(capacity);
        by_hash_.reserve(capacity * 2 + 1);
    }

    void clear() {
        candidates_.clear();
        by_hash_.clear();
        worst_ = MaxSegTree(capacity_);
    }

    void push(const BeamCandidate& candidate) {
        if (capacity_ == 0) {
            return;
        }

        if (const auto it = by_hash_.find(candidate.hash_key); it != by_hash_.end()) {
            const size_t index = it->second;
            if (candidate.evaluator < candidates_[index]->evaluator) {
                candidates_[index] = candidate;
                worst_.set(index, SlotValue{candidate.evaluator, index});
            }
            return;
        }

        if (candidates_.size() < capacity_) {
            const size_t index = candidates_.size();
            candidates_.push_back(candidate);
            by_hash_[candidate.hash_key] = index;
            worst_.set(index, SlotValue{candidate.evaluator, index});
            return;
        }

        const auto worst = worst_.max_all();
        if (!worst || candidate.evaluator >= worst->evaluator) {
            return;
        }

        const size_t index = worst->index;
        by_hash_.erase(candidates_[index]->hash_key);
        candidates_[index] = candidate;
        by_hash_[candidate.hash_key] = index;
        worst_.set(index, SlotValue{candidate.evaluator, index});
    }

    vector<BeamCandidate> take_sorted() {
        vector<BeamCandidate> result;
        result.reserve(candidates_.size());
        for (const auto& candidate : candidates_) {
            if (candidate) {
                result.push_back(*candidate);
            }
        }
        ranges::sort(result, [](const BeamCandidate& a, const BeamCandidate& b) {
            return tie(a.evaluator, a.hash_key) < tie(b.evaluator, b.hash_key);
        });
        clear();
        return result;
    }

private:
    size_t capacity_;
    vector<optional<BeamCandidate>> candidates_;
    unordered_map<HashKey, size_t> by_hash_;
    MaxSegTree worst_;
};

struct BeamNode {
    size_t parent = 0;
    optional<Action> action;
    Evaluator evaluator;
    HashKey hash_key = 0;
    size_t depth = 0;
};

enum class TourEdgeType {
    Forward,
    Backward,
    Visit,
};

struct TourEdge {
    TourEdgeType type;
    size_t node;
};

vector<size_t> path_from_root(const vector<BeamNode>& nodes, size_t node) {
    vector<size_t> path;
    while (node != 0) {
        path.push_back(node);
        node = nodes[node].parent;
    }
    ranges::reverse(path);
    return path;
}

vector<TourEdge> build_tour_edges(const vector<size_t>& leaves, const vector<BeamNode>& nodes) {
    vector<TourEdge> edges;
    vector<size_t> previous_path;

    for (const size_t leaf : leaves) {
        vector<size_t> path = path_from_root(nodes, leaf);
        size_t lcp = 0;
        while (lcp < previous_path.size() && lcp < path.size() && previous_path[lcp] == path[lcp]) {
            ++lcp;
        }

        for (size_t i = previous_path.size(); i > lcp; --i) {
            edges.push_back({TourEdgeType::Backward, previous_path[i - 1]});
        }
        for (size_t i = lcp; i < path.size(); ++i) {
            edges.push_back({TourEdgeType::Forward, path[i]});
        }
        edges.push_back({TourEdgeType::Visit, leaf});
        previous_path = move(path);
    }

    for (size_t i = previous_path.size(); i > 0; --i) {
        edges.push_back({TourEdgeType::Backward, previous_path[i - 1]});
    }
    return edges;
}

vector<Action> reconstruct_actions(const vector<BeamNode>& nodes, size_t node) {
    vector<Action> actions;
    while (node != 0) {
        actions.push_back(*nodes[node].action);
        node = nodes[node].parent;
    }
    ranges::reverse(actions);
    return actions;
}

vector<Action> rollback_beam_search(State state, size_t max_turn, size_t beam_width) {
    vector<BeamNode> nodes = {{0, nullopt, state.evaluate(0), state.hash_key(), 0}};
    vector<size_t> beam = {0};
    Selector selector(beam_width);

    for (size_t turn = 0; turn < max_turn; ++turn) {
        selector.clear();
        for (const TourEdge edge : build_tour_edges(beam, nodes)) {
            const BeamNode& node = nodes[edge.node];
            if (edge.type == TourEdgeType::Forward) {
                state.move_forward(*node.action);
            } else if (edge.type == TourEdgeType::Backward) {
                state.move_backward(*node.action);
            } else {
                assert(node.depth == turn);
                for (const Action action : state.enumerate_actions(turn)) {
                    state.move_forward(action);
                    selector.push({edge.node, action, state.evaluate(turn + 1), state.hash_key()});
                    state.move_backward(action);
                }
            }
        }

        const vector<BeamCandidate> selected = selector.take_sorted();
        if (selected.empty()) {
            break;
        }

        beam.clear();
        for (const BeamCandidate& candidate : selected) {
            const size_t depth = nodes[candidate.parent].depth + 1;
            nodes.push_back({candidate.parent, candidate.action, candidate.evaluator, candidate.hash_key, depth});
            beam.push_back(nodes.size() - 1);
        }
    }

    const size_t best_leaf = *ranges::min_element(beam, {}, [&](size_t node) {
        return nodes[node].evaluator;
    });
    return reconstruct_actions(nodes, best_leaf);
}

int main() {
    [[maybe_unused]] const vector<Action> actions = rollback_beam_search(State{}, 3, 4);
    return 0;
}
