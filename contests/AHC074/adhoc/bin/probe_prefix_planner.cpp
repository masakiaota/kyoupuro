// probe_prefix_planner.cpp
#define AHC_PROBE_TREES_LIBRARY
#include "probe_trees.cpp"

// 任意の接頭辞模様を持つ紙について、目標を作る可視紙列そのものをDPで選ぶ。
// 小幅では独立な全合法移動BFSと距離を照合し、復元した全操作も直接再生する。
// 公開Aを読む場合も先頭のバーコードだけを読み、ファイル後半の実注文は読まない。
// 例: target/release/probe_prefix_planner --check-width 3 --family bidirectional_full
//     target/release/probe_prefix_planner --simulate-width 64 --orders 4096 --placement 2

static uint64_t prefix_bits(int count) {
    assert(0 <= count && count <= 64);
    return count == 64 ? ~uint64_t{0} : (uint64_t{1} << count) - 1;
}

struct PrefixFrontier {
    vector<int> active;
    int distance = 0;
};

class PrefixPlanner {
    int N, K;
    vector<int> cover;
    vector<uint64_t> black;

public:
    int last_distance = 0;
    int64_t dp_states = 0, dp_transitions = 0, legal_moves = 0;
    int64_t active_sum = 0, mandatory_sum = 0, additional_sum = 0;

    explicit PrefixPlanner(const vector<string>& S)
        : N(int(S.front().size())), K(int(S.size())), cover(K), black(K) {
        assert(1 <= N && N <= 64);
        for (int k = 0; k < K; ++k) {
            assert(int(S[k].size()) == N);
            while (cover[k] < N && S[k][cover[k]] != '.') ++cover[k];
            assert(cover[k] > 0);
            for (int j = 0; j < N; ++j) {
                if (j >= cover[k]) assert(S[k][j] == '.');
                else assert(S[k][j] == '0' || S[k][j] == '1');
                if (S[k][j] == '1') black[k] |= uint64_t{1} << j;
            }
        }
        assert(*max_element(cover.begin(), cover.end()) == N);
    }

    uint64_t display(const vector<int>& order) const {
        int covered = 0;
        uint64_t result = 0;
        for (int k : order) if (cover[k] > covered) {
            result |= black[k] & (prefix_bits(cover[k]) ^ prefix_bits(covered));
            covered = cover[k];
        }
        assert(covered == N);
        return result;
    }

    int64_t future_run_cost(const vector<int>& order) const {
        // 各窓だけの一様2色MDPでは、先頭同色runの余分な紙数L_j-1が相対価値になる。
        // 全窓の和は同時変更と可視性を分離した近似であり、全体の厳密価値ではない。
        // 操作途中の候補も評価するので、targetではなく候補自身の表示色を使う。
        uint64_t unseen = prefix_bits(N), waiting = unseen, first_color = 0;
        int64_t value = -N;
        for (int k : order) {
            const uint64_t mask = prefix_bits(cover[k]);
            const uint64_t fresh = mask & unseen;
            first_color |= black[k] & fresh;
            unseen &= ~mask;
            const uint64_t opposite = (black[k] ^ first_color) & mask;
            value += popcount(waiting & mask & ~opposite);
            waiting &= ~opposite;
        }
        assert(unseen == 0 && value >= 0);
        return value;
    }

    PrefixFrontier frontier(const vector<int>& order, uint64_t target,
                            int tie_mode = 0, uint64_t tie_seed = 0) {
        assert(int(order.size()) == K);
        assert(0 <= tie_mode && tie_mode <= 2);
        vector<int> pos(K);
        for (int i = 0; i < K; ++i) pos[order[i]] = i;
        // sum[H,p]は被覆長<=Hかつ元位置<pの紙数。長さと位置の長方形をO(1)で数える。
        const int stride = K + 1;
        vector<int> sum((N + 1) * stride);
        for (int k = 0; k < K; ++k) ++sum[cover[k] * stride + pos[k] + 1];
        for (int H = 1; H <= N; ++H) for (int p = 1; p <= K; ++p)
            sum[H * stride + p] += sum[(H - 1) * stride + p] + sum[H * stride + p - 1]
                                         - sum[(H - 1) * stride + p - 1];
        auto count = [&](int low, int high, int first, int last) {
            if (low >= high || first >= last) return 0;
            first = max(first, 0); last = max(last, 0);
            return sum[high * stride + last] - sum[low * stride + last]
                 - sum[high * stride + first] + sum[low * stride + first];
        };
        vector<vector<int>> next(N);
        for (int L = 0; L < N; ++L) for (int k = 0; k < K; ++k) if (cover[k] > L) {
            const uint64_t interval = prefix_bits(cover[k]) ^ prefix_bits(L);
            if (((black[k] ^ target) & interval) == 0) next[L].push_back(k);
        }
        constexpr int infinity = 1 << 28;
        vector<int> best((N + 1) * stride, infinity), parent(best.size(), -1), paper(best.size(), -1);
        vector<int64_t> secondary(best.size(), numeric_limits<int64_t>::max());
        vector<int> tie_weight(K, 1);
        for (int k = 0; k < K; ++k) {
            if (tie_mode == 1) tie_weight[k] = -1;
            else if (tie_mode == 2) {
                // 未来の実注文と無関係な紙IDごとの固定重みで、同距離の可視紙列を多様化する。
                uint64_t x = uint64_t(k) + tie_seed + 0x9e3779b97f4a7c15ULL;
                x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
                x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
                x ^= x >> 31;
                tie_weight[k] = int(x % 2047) - 1023;
            }
        }
        // m+1を添字にする。初期m=-1は「まだ可視紙を選んでいない」。
        best[0] = 0;
        secondary[0] = 0;
        for (int L = 0; L < N; ++L) for (int mp = 0; mp <= K; ++mp) {
            const int state = L * stride + mp;
            if (best[state] == infinity) continue;
            ++dp_states;
            const int m = mp - 1;
            for (int k : next[L]) {
                ++dp_transitions;
                const int H = cover[k], p = pos[k], new_m = max(m, p);
                // 内側の長さでは既存mが必須移動を決め、Hと同長の紙には新しいmを使う。
                int added = count(L, H - 1, 0, m) + count(H - 1, H, 0, new_m);
                // 新たな可視親が非必須で、未移動の子が先行しているなら親を1枚追加する。
                if (p >= m && count(L, H - 1, m, p) > 0) ++added;
                const int to = H * stride + new_m + 1;
                // tie_mode=0は従来の(distance*(N+1)+可視紙数)と完全に同じ順序である。
                // 負の同点重みを距離と混ぜず、すべてのmodeで最短距離を第一目的に保つ。
                const int value = best[state] + added;
                const int64_t tie_value = secondary[state] + tie_weight[k];
                if (value < best[to] || (value == best[to] && tie_value < secondary[to])) {
                    best[to] = value;
                    secondary[to] = tie_value;
                    parent[to] = state;
                    paper[to] = k;
                }
            }
        }
        int goal = N * stride;
        for (int mp = 1; mp <= K; ++mp) {
            const int state = N * stride + mp;
            if (best[state] < best[goal] ||
                (best[state] == best[goal] && secondary[state] < secondary[goal])) goal = state;
        }
        if (best[goal] == infinity) throw runtime_error("target has no prefix frontier");
        PrefixFrontier result;
        result.distance = best[goal];
        for (int state = goal; state != 0; state = parent[state]) {
            assert(parent[state] >= 0 && paper[state] >= 0);
            result.active.push_back(paper[state]);
        }
        reverse(result.active.begin(), result.active.end());
        return result;
    }

    int distance_only(const vector<int>& order, uint64_t target) {
        return frontier(order, target).distance;
    }

    vector<pair<int, int>> plan(vector<int>& order, uint64_t target, int placement_mode = 0,
                               int frontier_tie_mode = 0, uint64_t tie_seed = 0) {
        const PrefixFrontier goal = frontier(order, target, frontier_tie_mode, tie_seed);
        last_distance = goal.distance;
        vector<int> pos(K), previous(K, -1);
        for (int i = 0; i < K; ++i) pos[order[i]] = i;
        vector<bool> active(K), mandatory(K), selected(K), remaining(K);
        int last_cover = 0;
        for (int v : goal.active) {
            active[v] = true;
            previous[v] = last_cover;
            last_cover = cover[v];
            for (int h = 0; h < K; ++h)
                if (cover[h] >= cover[v] && pos[h] < pos[v]) mandatory[h] = true;
        }
        selected = mandatory;
        for (int v : goal.active) if (!mandatory[v]) {
            for (int h = 0; h < K; ++h)
                if (!mandatory[h] && previous[v] < cover[h] && cover[h] < cover[v]
                    && pos[h] < pos[v]) selected[v] = true;
        }
        const int selected_count = int(count(selected.begin(), selected.end(), true));
        assert(selected_count == goal.distance);
        active_sum += goal.active.size();
        mandatory_sum += count(mandatory.begin(), mandatory.end(), true);
        additional_sum += selected_count - count(mandatory.begin(), mandatory.end(), true);
        auto ancestor = [&](int v, int h) {
            return active[v] && v != h && cover[h] > previous[v];
        };
        vector<int> route;
        for (int k : order) if (selected[k]) route.push_back(k);
        remaining = selected;
        vector<pair<int, int>> operations;
        for (int k : route) {
            const int at = int(find(order.begin(), order.end(), k) - order.begin());
            int before_cover = 0;
            for (int i = 0; i < at; ++i) before_cover = max(before_cover, cover[order[i]]);
            assert(cover[k] > before_cover);
            ++legal_moves;
            remaining[k] = false;
            vector<int> rest = order;
            rest.erase(rest.begin() + at);
            int lower = 0, upper = K - 1;
            for (int i = 0; i < K - 1; ++i) {
                const int h = rest[i];
                if (remaining[h]) {
                    if (cover[h] <= cover[k]) lower = max(lower, i + 1);
                } else {
                    if (ancestor(h, k)) lower = max(lower, i + 1);
                    if (ancestor(k, h)) upper = min(upper, i);
                }
            }
            assert(0 <= lower && lower <= upper && upper < K);
            int d = lower;
            if (placement_mode == 1) d = upper;
            else if (placement_mode == 2) {
                // 被覆長の逆転対だけを比較する。紙k以外の対は挿入位置によらない。
                int value = 0;
                for (int i = 0; i < K - 1; ++i)
                    value += i < lower ? cover[rest[i]] > cover[k] : cover[k] > cover[rest[i]];
                int best_value = value;
                for (int candidate = lower + 1; candidate <= upper; ++candidate) {
                    const int h = rest[candidate - 1];
                    value += (cover[h] > cover[k]) - (cover[k] > cover[h]);
                    if (value < best_value) { best_value = value; d = candidate; }
                }
            } else if (placement_mode == 3) {
                int64_t best_value = numeric_limits<int64_t>::max();
                for (int candidate = lower; candidate <= upper; ++candidate) {
                    auto next = rest;
                    next.insert(next.begin() + candidate, k);
                    const int64_t value = future_run_cost(next);
                    if (value < best_value) { best_value = value; d = candidate; }
                }
            }
            assert(d != at);
            rest.insert(rest.begin() + d, k);
            order = move(rest);
            operations.emplace_back(k, d);
        }
        assert(int(operations.size()) == goal.distance);
        assert(display(order) == target);
        return operations;
    }
};

vector<string> prefix_family(int N, const string& family) {
    vector<string> S;
    if (family == "cascade" || family == "reset_zero") {
        for (int k = 0; k < N; ++k) {
            string row(N, '.');
            for (int j = 0; j <= k; ++j) row[j] = j < k ? '1' : '0';
            S.push_back(row);
        }
        S.push_back(string(N, '1'));
        if (family == "reset_zero") S.push_back(string(N, '0'));
    } else if (family == "bidirectional_short" || family == "bidirectional_full") {
        const int last = family == "bidirectional_full" ? N : N - 1;
        for (int k = 0; k < last; ++k) for (int prefix = 0; prefix < 2; ++prefix) {
            string row(N, '.');
            for (int j = 0; j <= k; ++j) row[j] = char('0' + (j < k ? prefix : 1 - prefix));
            S.push_back(row);
        }
        S.push_back(string(N, '0'));
        S.push_back(string(N, '1'));
    } else if (family == "qary2") {
        assert(N % 2 == 0);
        for (int cell = 0; cell < N / 2; ++cell) for (int value = 0; value < 3; ++value) {
            string row(N, '.');
            for (int j = 0; j < cell * 2; ++j) row[j] = '1';
            for (int bit = 0; bit < 2; ++bit) row[cell * 2 + bit] = char('0' + (value >> bit & 1));
            S.push_back(row);
        }
        S.push_back(string(N, '1'));
    } else throw runtime_error("unknown family: " + family);
    return S;
}

vector<int> prefix_initial_order(int N, const string& family, const string& initial) {
    const int K = int(prefix_family(N, family).size());
    vector<int> order;
    if (initial == "natural") {
        order.resize(K);
        iota(order.begin(), order.end(), 0);
    } else if (initial == "cascade_subset" && family == "bidirectional_full") {
        // 元のcascadeを先頭へ置き、追加紙をその全1紙より後ろへ置く比較用初期状態。
        for (int j = 0; j < N; ++j) order.push_back(2 * j + 1);
        order.push_back(2 * N + 1);
        for (int j = 0; j < N; ++j) order.push_back(2 * j);
        order.push_back(2 * N);
    } else throw runtime_error("unsupported initial order: " + initial);
    assert(int(order.size()) == K);
    return order;
}

#ifndef AHC_PROBE_PREFIX_PLANNER_LIBRARY
int main(int argc, char** argv) {
    int check_width = 0, simulate_width = 0, orders = 4096, placement = 2;
    string family = "bidirectional_full", input, initial = "natural";
    bool balanced = false;
    for (int i = 1; i < argc; ++i) {
        string option = argv[i];
        if (option == "--balanced") { balanced = true; continue; }
        if (i + 1 >= argc) throw runtime_error("missing value");
        if (option == "--check-width") check_width = stoi(argv[++i]);
        else if (option == "--simulate-width") simulate_width = stoi(argv[++i]);
        else if (option == "--orders") orders = stoi(argv[++i]);
        else if (option == "--placement") placement = stoi(argv[++i]);
        else if (option == "--family") family = argv[++i];
        else if (option == "--input") input = argv[++i];
        else if (option == "--initial") initial = argv[++i];
        else throw runtime_error("unknown option: " + option);
    }
    assert(0 <= placement && placement <= 3);
    if (check_width) {
        const auto S = prefix_family(check_width, family);
        assert(S.size() <= 8);
        const Permutations permutations(int(S.size()));
        const TreeModel model(permutations, S);
        PrefixPlanner planner(S);
        int64_t checked = 0;
        for (int state = 0; state < model.states; ++state) for (int a = 0; a < model.P; ++a) {
            vector<int> order(permutations.order[state].begin(),
                              permutations.order[state].begin() + model.K);
            const auto ops = planner.plan(order, uint64_t(a), placement);
            if (int(ops.size()) != model.distance[a][state]) {
                cerr << "distance mismatch state=" << state << " target=" << a
                     << " DP=" << ops.size() << " BFS=" << int(model.distance[a][state]) << '\n';
                return 1;
            }
            ++checked;
        }
        cout << "check family=" << family << " N=" << check_width << " K=" << S.size()
             << " pairs=" << checked << " legal_moves=" << planner.legal_moves
             << " dp_transitions=" << planner.dp_transitions << " mismatches=0\n";
    }
    if (simulate_width) {
        int N = simulate_width;
        vector<uint64_t> A;
        if (!input.empty()) {
            ifstream stream(input);
            int K, P, R, input_N;
            stream >> input_N >> K >> P >> R;
            assert(stream && input_N == N);
            for (int v = 0; v < P; ++v) {
                string row; stream >> row;
                assert(int(row.size()) == N);
                uint64_t bits = 0;
                for (int j = 0; j < N; ++j) if (row[j] == '1') bits |= uint64_t{1} << j;
                A.push_back(bits);
            }
        }
        const auto S = prefix_family(N, family);
        PrefixPlanner planner(S);
        vector<int> order = prefix_initial_order(N, family, initial);
        mt19937_64 rng(20261031);
        int previous = -1;
        int64_t T = 0;
        const auto started = chrono::steady_clock::now();
        for (int r = 0; r < orders; ++r) {
            uint64_t target = 0;
            if (!A.empty()) {
                int a = int(rng() % (A.size() - (previous >= 0)));
                if (previous >= 0 && a >= previous) ++a;
                previous = a;
                target = A[a];
            } else if (balanced) {
                vector<int> windows(N); iota(windows.begin(), windows.end(), 0);
                for (int j = 0; j < N / 2; ++j) {
                    const int other = j + int(rng() % (N - j));
                    swap(windows[j], windows[other]);
                    target |= uint64_t{1} << windows[j];
                }
            } else target = rng() & prefix_bits(N);
            T += planner.plan(order, target, placement).size();
        }
        const double seconds = chrono::duration<double>(chrono::steady_clock::now() - started).count();
        cout << fixed << setprecision(9) << "simulate family=" << family << " N=" << N
             << " K=" << S.size() << " orders=" << orders << " placement=" << placement
             << " initial=" << initial
             << " T=" << T << " mean=" << double(T) / orders << " active_mean="
             << double(planner.active_sum) / orders << " mandatory_mean="
             << double(planner.mandatory_sum) / orders << " additional_mean="
             << double(planner.additional_sum) / orders << " seconds=" << seconds << '\n';
    }
    if (!check_width && !simulate_width) throw runtime_error("choose --check-width or --simulate-width");
}
#endif
