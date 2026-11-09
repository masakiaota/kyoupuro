// probe_laminar_repeated_moves.cpp
// B114: private窓付きlaminarでも、同じ紙の再移動が最短経路に必要かを判別する。
// 3窓の固定族を全合法BFSし、各紙1回に制限した別BFSと比較する。
#include <bits/stdc++.h>
using namespace std;
struct Paper { unsigned mask, black; };
struct State { vector<int> order; unsigned used = 0; int parent = -1, k = -1, d = -1, distance = 0; };
struct Result { int distance = -1, states = 0; vector<pair<int,int>> operations; };
static unsigned visible(const vector<Paper>& S, const vector<int>& order) {
    // windowを個別に走査し、bitmask unionによる実装とは独立に可視紙を求める。
    unsigned answer = 0;
    for (int j = 0; j < 3; ++j) for (int k : order) if (S[k].mask >> j & 1) {
        answer |= 1u << k; break;
    }
    return answer;
}
static unsigned display(const vector<Paper>& S, const vector<int>& order) {
    unsigned answer = 0;
    for (int j = 0; j < 3; ++j) for (int k : order) if (S[k].mask >> j & 1) {
        answer |= ((S[k].black >> j) & 1u) << j; break;
    }
    return answer;
}
static uint64_t key(const State& s, bool once) {
    uint64_t answer = once ? uint64_t(s.used) << 24 : 0;
    for (int p = 0; p < int(s.order.size()); ++p) answer |= uint64_t(s.order[p]) << (3*p);
    return answer;
}
static Result solve(const vector<Paper>& S, bool once) {
    const int K = int(S.size());
    State initial; initial.order.resize(K); iota(initial.order.begin(),initial.order.end(),0);
    vector<State> queue{initial}; unordered_set<uint64_t> seen{key(initial,once)};
    for (int head = 0; head < int(queue.size()); ++head) {
        const State state = queue[head];
        if (display(S,state.order) == 0) {
            Result result; result.distance = state.distance; result.states = int(queue.size());
            for (int at = head; queue[at].parent >= 0; at = queue[at].parent)
                result.operations.emplace_back(queue[at].k,queue[at].d);
            reverse(result.operations.begin(),result.operations.end());
            return result;
        }
        const unsigned view = visible(S,state.order);
        for (int p = 0; p < K; ++p) {
            const int k = state.order[p];
            if (!(view >> k & 1) || (once && (state.used >> k & 1))) continue;
            for (int d = 0; d < K; ++d) if (d != p) {
                State next = state;
                next.order.erase(next.order.begin()+p); next.order.insert(next.order.begin()+d,k);
                next.used |= 1u << k; next.parent = head; next.k = k; next.d = d; ++next.distance;
                if (seen.insert(key(next,once)).second) queue.push_back(std::move(next));
            }
        }
    }
    return {-1,int(queue.size()),{}};
}
static void check(const vector<Paper>& S, const Result& result, bool once) {
    vector<int> order(S.size()); iota(order.begin(),order.end(),0);
    vector<int> count(S.size());
    for (auto [k,d] : result.operations) {
        unsigned covered = 0;
        const auto at = find(order.begin(),order.end(),k);
        assert(at != order.end());
        for (auto i = order.begin(); i != at; ++i) covered |= S[*i].mask;
        assert((S[k].mask & ~covered) != 0);
        assert(0 <= d && d < int(S.size()));
        order.erase(at); order.insert(order.begin()+d,k);
        ++count[k]; if (once) assert(count[k] == 1);
    }
    unsigned covered = 0, color = 0;
    for (int k : order) { color |= S[k].black & ~covered; covered |= S[k].mask; }
    assert(color == 0 && int(result.operations.size()) == result.distance);
}
int main() {
    for (int duplicates = 1; duplicates <= 4; ++duplicates) {
        vector<Paper> S{{7,1}}; // c=100。窓の文字列順はbit 0から。
        for (int q = 0; q < duplicates; ++q) S.push_back({2,2}); // q=.1.
        S.push_back({1,0}); // a=0..
        const auto unrestricted = solve(S,false), once = solve(S,true);
        check(S,unrestricted,false); check(S,once,true);
        assert(unrestricted.distance == min(duplicates+1,3));
        assert(once.distance == duplicates+1);
        cout << "{\"q_papers\":" << duplicates << ",\"K\":" << S.size()
             << ",\"BFS\":" << unrestricted.distance << ",\"once_BFS\":" << once.distance
             << ",\"BFS_states\":" << unrestricted.states << ",\"once_states\":" << once.states
             << ",\"route\":[";
        for (int i = 0; i < int(unrestricted.operations.size()); ++i) {
            if (i) cout << ',';
            cout << '[' << unrestricted.operations[i].first << ',' << unrestricted.operations[i].second << ']';
        }
        cout << "]}\n";
    }
}
