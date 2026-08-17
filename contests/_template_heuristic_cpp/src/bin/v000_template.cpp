// v000_template.cpp
#include <bits/stdc++.h>

using namespace std;

/// AtCoder 側の基準の探索打ち切り秒数。コンテストごとに調整する。
constexpr double JUDGE_TIME_LIMIT_SEC = 1.90;
/// LOCAL 時はローカル実行の速度差を見込んで探索時間を短くする。
constexpr double LOCAL_TIME_RATIO = 0.80;

#ifdef LOCAL
constexpr double PROGRAM_TIME_LIMIT_SEC = JUDGE_TIME_LIMIT_SEC * LOCAL_TIME_RATIO;
#else
constexpr double PROGRAM_TIME_LIMIT_SEC = JUDGE_TIME_LIMIT_SEC;
#endif

#ifdef LOCAL
struct TraceStats {
    size_t fallback_count = 0;
    map<string, int64_t> counts;
    map<string, double> times_ms;

    void mark_fallback() {
        ++fallback_count;
    }

    void count(const string& key) {
        count_by(key, 1);
    }

    void count_by(const string& key, int64_t delta) {
        counts[key] += delta;
    }

    void add_time_ms(const string& key, double ms) {
        times_ms[key] += ms;
    }

    void summary() const {
        cerr << "[summary] fallback_count=" << fallback_count << '\n';
        for (const auto& [key, value] : counts) {
            cerr << "[summary.count] " << key << '=' << value << '\n';
        }
        const auto old_flags = cerr.flags();
        const auto old_precision = cerr.precision();
        cerr << fixed << setprecision(3);
        for (const auto& [key, value] : times_ms) {
            cerr << "[summary.time_ms] " << key << '=' << value << '\n';
        }
        cerr.flags(old_flags);
        cerr.precision(old_precision);
    }
};

template <class F>
decltype(auto) local_time_impl(TraceStats& trace, const string& key, F&& body) {
    const auto start = chrono::steady_clock::now();
    if constexpr (is_void_v<invoke_result_t<F>>) {
        invoke(forward<F>(body));
        const double ms = chrono::duration<double, milli>(chrono::steady_clock::now() - start).count();
        trace.add_time_ms(key, ms);
    } else {
        auto result = invoke(forward<F>(body));
        const double ms = chrono::duration<double, milli>(chrono::steady_clock::now() - start).count();
        trace.add_time_ms(key, ms);
        return result;
    }
}

#define LOCAL_ONLY(...) do { __VA_ARGS__; } while (false)
#define LOCAL_TIME(trace, key, ...) local_time_impl((trace), (key), (__VA_ARGS__))
#else
#define LOCAL_ONLY(...) do { } while (false)
#define LOCAL_TIME(trace, key, ...) (__VA_ARGS__)()
#endif

class TimeKeeper {
public:
    /// check_interval_log2 = 8 なら 2^8 = 256 反復ごとに時計更新する。
    explicit TimeKeeper(double time_limit_sec, uint32_t check_interval_log2)
        : start_(chrono::steady_clock::now()), time_limit_sec_(time_limit_sec) {
        assert(time_limit_sec > 0.0);
        assert(check_interval_log2 < 63);
        check_mask_ = check_interval_log2 == 0 ? 0 : (uint64_t{1} << check_interval_log2) - 1;
        force_update();
    }

    /// ホットループではこれだけ呼ぶ。true なら継続、false なら打ち切る。
    [[gnu::always_inline]] bool step() {
        ++iter_;
        if ((iter_ & check_mask_) == 0) {
            force_update();
        }
        return !is_over_;
    }

    /// 明示的に時計を更新したいときに使う。
    [[gnu::always_inline]] void force_update() {
        const double elapsed = exact_elapsed_sec();
        elapsed_sec_ = elapsed;
        progress_ = clamp(elapsed / time_limit_sec_, 0.0, 1.0);
        is_over_ = elapsed >= time_limit_sec_;
    }

    /// batched な経過時間。
    [[gnu::always_inline]] double elapsed_sec() const {
        return elapsed_sec_;
    }

    /// batched な進捗率 [0, 1]。
    [[gnu::always_inline]] double progress() const {
        return progress_;
    }

    /// batched な時間切れ判定。
    [[gnu::always_inline]] bool is_time_over() const {
        return is_over_;
    }

    /// ログ用の正確な経過時間。
    double exact_elapsed_sec() const {
        return chrono::duration<double>(chrono::steady_clock::now() - start_).count();
    }

    /// ログ用の正確な残り時間。
    double exact_remaining_sec() const {
        return max(0.0, time_limit_sec_ - exact_elapsed_sec());
    }

private:
    chrono::steady_clock::time_point start_;
    double time_limit_sec_;
    uint64_t iter_ = 0;
    uint64_t check_mask_ = 0;
    double elapsed_sec_ = 0.0;
    double progress_ = 0.0;
    bool is_over_ = false;
};

int main() {
    // TimeKeeper は main 開始直後に作り、探索打ち切りには PROGRAM_TIME_LIMIT_SEC を使う。
    // フェーズ切替などの時間系パラメータは PROGRAM_TIME_LIMIT_SEC に対する割合で指定する。
    [[maybe_unused]] TimeKeeper time_keeper(PROGRAM_TIME_LIMIT_SEC, 8);
    return 0;
}
