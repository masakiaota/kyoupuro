// v0000_template.cpp
// 速度優先の共通指定。AtCoder のビルドオプションは変えられないので、ソース先頭で補う。
#pragma GCC optimize("O3,unroll-loops")
#if defined(__x86_64__)
#pragma GCC target("avx2,bmi,bmi2,popcnt,lzcnt")
#endif
// assert は使わない (速度優先)。出力の合法性は公式 tester や scorer で検査する。
#ifndef NDEBUG
#define NDEBUG
#endif
#include <bits/stdc++.h>

using namespace std;

/// AtCoder 側の基準の探索打ち切り秒数。コンテストごとに調整する。
constexpr double JUDGE_TIME_LIMIT_SEC = 1.90;
/// 未指定時と提出時は補正しない。LOCAL の倍率はビルド時に渡す。
#if defined(LOCAL) && defined(AHC_LOCAL_TIME_RATIO)
constexpr double LOCAL_TIME_RATIO = AHC_LOCAL_TIME_RATIO;
#else
constexpr double LOCAL_TIME_RATIO = 1.0;
#endif
static_assert(LOCAL_TIME_RATIO > 0.0 && LOCAL_TIME_RATIO <= numeric_limits<double>::max());

/// process CPU 時間（user+system）の時計。ジャッジ応答の待ちや、並列評価で CPU を
/// 割り当てられていない時間は進まない。
struct ProcessCpuClock {
    using rep = int64_t;
    using period = nano;
    using duration = chrono::duration<rep, period>;
    using time_point = chrono::time_point<ProcessCpuClock>;
    static constexpr bool is_steady = true;

    static time_point now() noexcept {
        timespec value{};
        if (clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &value) != 0) abort();
        return time_point(duration(int64_t(value.tv_sec) * 1000000000LL + value.tv_nsec));
    }
};

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

/// 経過時間は main 開始からの process CPU 時間で数える。ジャッジ応答の待ちは CPU を使わないので含まれず、
/// 並列評価でも探索予算が削られない。入力の解析に使った CPU 時間は solver の費用として数える。
class TimeKeeper {
    using Clock = ProcessCpuClock;
    using WaitClock = chrono::steady_clock;
    // 時計の単位変換と端末補正をまとめ、時計更新時の割り算をなくす。
    static constexpr double TICK_TO_JUDGE_SEC =
        (double(Clock::period::num) / double(Clock::period::den)) / LOCAL_TIME_RATIO;
    static_assert(TICK_TO_JUDGE_SEC > 0.0 && TICK_TO_JUDGE_SEC <= numeric_limits<double>::max());

public:
    /// 引数と返す時間は AtCoder 相当の秒。既定は 2^10 = 1024 反復ごとに更新する。
    /// 1 反復が重い場合は更新間隔を短くする。0 なら毎反復更新する。
    explicit TimeKeeper(double time_limit_sec, uint32_t check_interval_log2 = 10)
        : start_(Clock::now()), time_limit_sec_(time_limit_sec),
          inverse_time_limit_(1.0 / time_limit_sec) {
        check_mask_ = check_interval_log2 == 0 ? 0 : (uint64_t{1} << check_interval_log2) - 1;
        force_update();
    }
    // 終了時の報告行が複製ごとに出ないよう、値渡しを禁止して参照で渡させる。
    TimeKeeper(const TimeKeeper&) = delete;
    TimeKeeper& operator=(const TimeKeeper&) = delete;

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
        progress_ = clamp(elapsed * inverse_time_limit_, 0.0, 1.0);
        is_over_ = elapsed >= time_limit_sec_;
    }

    /// 最後の時計更新時点の経過秒数。取得だけでは時計を更新しない。
    [[gnu::always_inline]] double elapsed_sec() const {
        return elapsed_sec_;
    }

    /// 最後の時計更新時点の残り秒数。
    [[gnu::always_inline]] double remaining_sec() const {
        return max(0.0, time_limit_sec_ - elapsed_sec_);
    }

    /// batched な進捗率 [0, 1]。
    [[gnu::always_inline]] double progress() const {
        return progress_;
    }

    /// batched な時間切れ判定。
    [[gnu::always_inline]] bool is_time_over() const {
        return is_over_;
    }

    /// ジャッジから読む処理を包み、待ちの実時間を診断用に記録する。CPU 時計からは差し引かない。
    /// インタラクティブ問題では初期入力も含め全読み込みに使う。非インタラクティブ問題では使わなくてよい。
    /// 例: time_keeper.wait_judge([&] { cin >> a; });
    template <class Read>
    decltype(auto) wait_judge(Read&& read) {
        const auto begin = WaitClock::now();
        if constexpr (is_void_v<invoke_result_t<Read>>) {
            invoke(forward<Read>(read));
            wait_ticks_ += (WaitClock::now() - begin).count();
        } else {
            auto result = invoke(forward<Read>(read));
            wait_ticks_ += (WaitClock::now() - begin).count();
            return result;
        }
    }

    /// 時計をその場で読み取った AtCoder 相当の経過秒数。
    double exact_elapsed_sec() const {
        return double((Clock::now() - start_).count()) * TICK_TO_JUDGE_SEC;
    }

    /// 時計をその場で読み取った AtCoder 相当の残り秒数。
    double exact_remaining_sec() const {
        return max(0.0, time_limit_sec_ - exact_elapsed_sec());
    }

    /// 評価側は実時間ではなくこの行の solver_ms で時間切れを判定する (scripts/eval.py)。
    /// solver_ms は端末補正前の process CPU ミリ秒、wait_ms は wait_judge() 内の実時間（診断用）である。
    /// AtCoder では stderr は採点に使われない。
    ~TimeKeeper() {
        constexpr double cpu_tick_ms = 1000.0 * double(Clock::period::num) / double(Clock::period::den);
        constexpr double wait_tick_ms = 1000.0 * double(WaitClock::period::num) / double(WaitClock::period::den);
        const auto total_ticks = (Clock::now() - start_).count();
        cerr << "time_keeper: solver_ms=" << llround(double(total_ticks) * cpu_tick_ms)
             << " wait_ms=" << llround(double(wait_ticks_) * wait_tick_ms) << '\n';
    }

private:
    Clock::time_point start_;
    double time_limit_sec_;
    double inverse_time_limit_;
    WaitClock::rep wait_ticks_ = 0;
    uint64_t iter_ = 0;
    uint64_t check_mask_ = 0;
    double elapsed_sec_ = 0.0;
    double progress_ = 0.0;
    bool is_over_ = false;
};

int main() {
    // main 開始直後から計測する。時計は process CPU 時間である。終了時刻やフェーズ切替は AtCoder 基準の秒で指定する。
    // 探索ループの step() で時計を更新し、最新の値が必要な箇所では force_update() を呼ぶ。
    TimeKeeper time_keeper(JUDGE_TIME_LIMIT_SEC);
    return 0;
}
