// v0000_template.cpp
// 速度優先の共通指定。AtCoder のビルドオプションは変えられないので、ソース先頭で補う。
#pragma GCC optimize("O3,unroll-loops")
#if defined(__x86_64__)
#pragma GCC target("avx2,bmi,bmi2,popcnt,lzcnt")
#endif
// assert は使わない（速度優先）。出力の合法性は公式 tester が検査する。
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
/// 往復の遅い実験環境や並列評価でも探索予算が削られない。入力の解析に使った CPU 時間は solver の費用として数える。
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
    /// 初期入力も含め全読み込みに使う。例: time_keeper.wait_judge([&] { cin >> a; });
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
    /// solver_ms は端末補正前の process CPU ミリ秒、wait_ms は読み込み中の実時間（診断用）である。
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

/// 乱数生成器 xoshiro256++。同じ seed ならどの環境でも同じ列になる。
/// std::shuffle などの標準アルゴリズムにもそのまま渡せる。
struct Rng {
    using result_type = uint64_t;

    explicit Rng(uint64_t seed = 0) {
        // splitmix64 で 256 bit の状態を初期化する。
        for (uint64_t& x : s_) {
            seed += 0x9E3779B97F4A7C15ULL;
            uint64_t z = seed;
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
            x = z ^ (z >> 31);
        }
    }

    static constexpr result_type min() { return 0; }
    static constexpr result_type max() { return numeric_limits<result_type>::max(); }
    [[gnu::always_inline]] result_type operator()() { return next_u64(); }

    /// 64 bit の一様乱数。
    [[gnu::always_inline]] uint64_t next_u64() {
        const uint64_t result = rotl(s_[0] + s_[3], 23) + s_[0];
        const uint64_t t = s_[1] << 17;
        s_[2] ^= s_[0];
        s_[3] ^= s_[1];
        s_[1] ^= s_[2];
        s_[0] ^= s_[3];
        s_[2] ^= t;
        s_[3] = rotl(s_[3], 45);
        return result;
    }

    /// [0, n) の整数（n > 0）。割り算を使わず、掛け算と上位 64 bit で作る。
    [[gnu::always_inline]] int next_int(int n) {
        return int((static_cast<unsigned __int128>(next_u64()) * uint64_t(n)) >> 64);
    }

    /// [0, 1) の実数。
    [[gnu::always_inline]] double next_double() {
        return double(next_u64() >> 11) * 0x1.0p-53;
    }

private:
    array<uint64_t, 4> s_{};
};
static_assert(uniform_random_bit_generator<Rng>);

/// 初期情報。記号は問題文と同じ。
struct Input {
    int N = 0, K = 0, P = 0, R = 0;
    vector<string> A;  // A[v][j] は '0'（白）/ '1'（黒）
};

/// 1 回の操作。紙 k を取り出し、残りの紙の上から d 枚の下へ挿入する。
struct Operation {
    int k = 0;
    int d = 0;
};

/// 初期情報を読む。
Input read_input(TimeKeeper& time_keeper) {
    Input input;
    time_keeper.wait_judge([&] {
        cin >> input.N >> input.K >> input.P >> input.R;
        input.A.resize(input.P);
        for (string& row : input.A) cin >> row;
    });
    return input;
}

/// 紙の設計 S[0..K-1] を出力して flush する。
void write_design(const vector<string>& S) {
    for (const string& row : S) cout << row << '\n';
    cout.flush();
}

/// 注文 a[r] を読む。-1 または入力の終わりなら nullopt を返す。呼び出し側は直ちに終了する。
optional<int> read_order(TimeKeeper& time_keeper) {
    int a = -1;
    const bool ok = time_keeper.wait_judge([&] { return bool(cin >> a); });
    if (!ok || a < 0) return nullopt;
    return a;
}

/// m と各行 "k d" を出力して flush する。
void write_operations(const vector<Operation>& operations) {
    cout << operations.size() << '\n';
    for (const auto& [k, d] : operations) cout << k << ' ' << d << '\n';
    cout.flush();
}

int main() {
    // main 開始直後から計測する。終了時刻やフェーズ切替は AtCoder 基準の秒で指定する。
    // 探索ループの step() で時計を更新し、最新の値が必要な箇所では force_update() を呼ぶ。
    // ジャッジからの読み込みは初期入力も含めすべて wait_judge() で包む。
    TimeKeeper time_keeper(JUDGE_TIME_LIMIT_SEC);
    // 対話型なので cin と cout の連動を外し、出力関数ごとに明示的に flush する。
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    const Input input = read_input(time_keeper);
    // 仮の中身: 全部穴の紙を出力し、各注文に 0 手で答える（合法だが全注文が誤注文になる）。
    const vector<string> S(input.K, string(input.N, '.'));
    write_design(S);
    for (int r = 0; r < input.R; ++r) {
        if (!read_order(time_keeper)) return 0;
        write_operations({});
    }
    return 0;
}
