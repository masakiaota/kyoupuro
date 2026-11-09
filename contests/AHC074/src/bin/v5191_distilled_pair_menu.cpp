// v5191_distilled_pair_menu.cpp
#if defined(__GNUC__) && !defined(__clang__)
#pragma GCC optimize("O3","no-reorder-blocks-and-partition")
#endif
// AHC074。C++23、逐次入力、単一スレッドで動作する提出版。
// 相補補助で支えるペアと通常の共同予約ペアを組み合わせ、残余窓の表示と
// 移動紙の削減、次注文へ残す積層順を評価する。各出力操作は再生検査する。
// 初期の公開商品表で紙を選び、注文後は先頭2枚の取得条件から移動集合を探索する。
#include <bits/stdc++.h>
#include <cassert>
#if defined(__x86_64__) && defined(__GNUC__)
#include <immintrin.h>
#endif
#if defined(__AVX2__)
#include <immintrin.h>
#endif
using namespace std;
#if defined(__x86_64__) && defined(__GNUC__) && !defined(__clang__)
#define AHC_ARCH __attribute__((target_clones("arch=x86-64-v3","default")))
#else
#define AHC_ARCH
#endif

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

#ifdef LOCAL
TraceStats trace_stats;
#endif

using U=uint64_t;
constexpr int N=64,P=256,K=192;
struct Rng {
 unsigned x=39123591;
 unsigned operator()(){x^=x<<13;x^=x>>17;return x^=x<<5;}
 int operator()(int n){return (*this)()%n;}
} rng;
inline int pc(U a) {
#if defined(__x86_64__) && defined(__GNUC__) && !defined(__POPCNT__)
  if (__builtin_cpu_supports("popcnt")) {
    U n;
    asm("popcntq %1,%0" : "=r"(n) : "r"(a));
    return static_cast<int>(n);
  }
#endif
  return __builtin_popcountll(a);
}
int ctz(U a){return __builtin_ctzll(a);}
U lo(int n){return n==64?~0ULL:(1ULL<<n)-1;}
struct Move{uint8_t k,d;};
struct Card{U m,v;int end;};
struct State{
 array<uint8_t,K>a;
 vector<Move>op;
 int front=0;U cover=0;
};
// 通常ペアでは既存商品の土台、相補ペアでは補助紙を用いる。
constexpr int MaxGroups=7,MaxDonors=2*MaxGroups,DonorHead=32;
int DonorGroups=6,DonorCount=12;
void setGroupCount(int g){DonorGroups=g;DonorCount=2*g;}
U A[P+MaxDonors],low;
array<U,MaxDonors> donorMask;
int activeDonorPairs=0;
Card cards[K];int home[K],usage[K];
array<vector<int>,P> core;
int H;
// 紙設計時の費用だけを変え、受注時の評価・時間配分を共通にする。
constexpr array<int,4> layout_prices{25,19,4,55};
// V6064_JOINT_BACKING_BEGIN
struct PatchGroup{vector<int> a,c;U common;};vector<PatchGroup> patches;
#ifdef LOCAL
struct JointRoleInfo {
 unsigned seed=0;
 uint64_t pre=0,post=0;
 int reserved=0;
} joint_role_info;
#endif
// v5022_partner_options.hpp
// 候補辺に同じ二商品が別の符号群で現れても、別の相方として数えない。
// v5023_observed_sheets.hpp
// 完成した対応する親候補の印字を現在の窓順に写す。提出時には同じ値を親候補から直接渡せる。
namespace observed_sheets {
array<U,K> mask{},ink{};array<uint8_t,K> grip{};
array<array<int16_t,2>,P> anchor{};array<int16_t,P> partner{};array<float,P> item{};
U remap(U w,const array<int,N>& order){
 U result=0;for(int z=0;z<N;++z)result|=((w>>order[z])&1ULL)<<z;return result;
}
struct Quote {
 bool ready=false;array<uint8_t,2> plain{};array<array<uint8_t,32>,2> head{};
 void prepare(int a,int b){
  if(ready)return;ready=true;U tail=(A[a]^A[b])&~low;
  for(int side=0;side<2;++side){
   int product=side?b:a;U target=A[product];array<U,192> bucket{};int best=191;
   for(int k=0;k<K;++k){
    if(k==anchor[a][0]||k==anchor[b][0]||k==anchor[a][1]||k==anchor[b][1])continue;
    U wrong=mask[k]&(ink[k]^target);int error=pc(tail&wrong),holes=pc(tail&~mask[k]);
    int cost=3*holes+5*error+min(16,int(grip[k]))/2;
    best=min(best,cost);bucket[cost]|=mask[k]&~wrong&low;
   }
   plain[side]=uint8_t(best);head[side].fill(191);U open=low;
   for(int c=best;c<192&&open;++c){
    U fixed=open&bucket[c];open&=~fixed;
    while(fixed){int z=ctz(fixed);head[side][z]=uint8_t(c);fixed&=fixed-1;}
   }
  }
 }
 int erasure(int z,int mode)const{
  int a=head[0][z],b=head[1][z],extra=a+b-plain[0]-plain[1];
  if(mode==5)return clamp(3+extra/2,2,28);
  if(mode==6)return clamp(1+extra/3+(a+b)/8,2,28);
  if(mode==7)return clamp(5+extra+max(a,b)/10,3,32);
  return clamp(2+extra/2+(a+b)/16,2,24);
 }
 int pair_charge(int a,int b,int mode)const{
  int existing=int(lround(2.0*(item[a]+item[b])));
  if(mode==5)return clamp(-existing,-14,14);
  if(mode==6)return clamp((plain[0]+plain[1])/3-8-existing/2,-16,16);
  if(mode==7){
   int disruption=partner[a]==b?0:clamp(4-existing/2,0,12);
   return clamp((plain[0]+plain[1])/4-7+disruption-existing/2,-14,18);
  }
  return clamp((plain[0]+plain[1])/3-9-existing,-20,20);
 }
};
}

namespace price_experiment {
int repair=0,share=0;
struct TwoPartners {
 int id[2]={-1,-1},cost[2]={100000,100000};
 void add(int partner,int price){
  if(partner==id[0]){cost[0]=min(cost[0],price);return;}
  if(partner==id[1])cost[1]=min(cost[1],price);
  else if(price<cost[1]){id[1]=partner;cost[1]=price;}
  if(cost[1]<cost[0]){swap(cost[0],cost[1]);swap(id[0],id[1]);}
 }
 int alternative(int partner,int cap)const{return min(cap,cost[id[0]==partner]);}
};
struct RepairQuality {
 bool ready=false;array<array<uint8_t,32>,2> mismatch{};
 void prepare(int a,int b){
  if(ready)return;ready=true;
  U tail=(A[a]^A[b])&~low;
  for(int side=0;side<2;++side){
   U target=A[side?b:a];array<U,33> accepted{};
   for(int c=0;c<P;++c)if(c!=a&&c!=b){
    U disagreement=A[c]^target;
    accepted[pc(disagreement&tail)]|=~disagreement&low;
   }
   U open=low;
   for(int e=0;e<=32&&open;++e){
    U found=open&accepted[e];open&=~found;
    while(found){int z=ctz(found);mismatch[side][z]=uint8_t(e);found&=found-1;}
   }
  }
 }
 int price(int z)const{
  int a=mismatch[0][z],b=mismatch[1][z];
  if(repair==1)return 2+2*(a+b);
  if(repair==2)return 1+a+b;
  if(repair==3)return 3+3*max(a,b);
  return 1+2*min(a,b)+max(a,b);
 }
};
}

namespace joint6064 {
static pair<int,int> forbiddenPair={-1,-1};
constexpr int infinity=1000000000;
struct Client {int product;U differences;};
// 印刷できる端点の最小値Hとは別に、土台を選ぶときの穴の重みを位置で変える。
const int reachSetting=11022;
U priceBits[2][4]{},hazardBits[4]{};
const int hazardSetting=108;bool hazardOn=true;

int hazardCost(U m){if(!hazardOn)return 0;return pc(m&hazardBits[0])+2*pc(m&hazardBits[1])+4*pc(m&hazardBits[2])+8*pc(m&hazardBits[3]);}

void reachPrices(bool observed){
 const int last=reachSetting/10%10,first=reachSetting/100%100,power=reachSetting%10;
 int total[2]{},counts[2][64]{},late[64]{};
 if(observed){

  for(auto& c:core){
   int s=c.size()==1?0:1;
   int reach=s?min(cards[c[0]].end,cards[c[1]].end):cards[c[0]].end;
   ++total[s];for(int z=0;z<=reach;z++)counts[s][z]++;
   if(s)for(int z=cards[c[0]].end+1;z<=cards[c[1]].end;z++)++late[z];
  }
 }else{

  // 初期分布は端点選択前の棚に合わせる。
  array<int,N> nominal{};nominal[35]=6;for(int z=36;z<N;z++)nominal[z]=1+(z==63);
  int extra=K-accumulate(nominal.begin(),nominal.end(),0);
  for(int i=0;i<extra;i++){int z=36+min(27,int(28*pow((i+.5)/extra,100./155)));nominal[z]++;}
  total[0]=K;total[1]=K*K;
  for(int z=63,zsum=0;z>=0;z--){zsum+=nominal[z];counts[0][z]=zsum;counts[1][z]=zsum*zsum;late[z]=zsum*(K-zsum);}
 }
 fill(hazardBits,hazardBits+4,0);
 for(int z=36;z<N;z++){
  int w=clamp(int(lround(double(hazardSetting%100)*late[z]/max(1,total[1]))),0,15);

  for(int j=0;j<4;j++)if(w>>j&1)hazardBits[j]|=U(1)<<z;
 }
 for(int s=0;s<2;s++){
  fill(priceBits[s],priceBits[s]+4,0);
  for(int z=0;z<N;z++){
   double prob=double(counts[s][z])/max(1,total[s]);
   int w=z<36?12:int(lround(last+(first-last)*pow(prob,power)));
   for(int bit=0;bit<4;bit++)if(w>>bit&1)priceBits[s][bit]|=U(1)<<z;
  }
 }
}
inline int weighted(U x,int role){
 auto& m=priceBits[role];
 return pc(x&m[0])+2*pc(x&m[1])+4*pc(x&m[2])+8*pc(x&m[3]);
}

struct PairMetric {
 alignas(64) int16_t secondary[P][P]{},both_prices[P][P]{};
 alignas(64) int8_t head_ones[P][P]{},tail_ones[P][P]{};
 void build(){
  for(int a=0;a<P;a++)for(int b=a+1;b<P;b++){
   U diff=A[a]^A[b];int secondary_price=weighted(diff,1);
   secondary[a][b]=secondary[b][a]=secondary_price;
   both_prices[a][b]=both_prices[b][a]=secondary_price+weighted(diff,0);
   head_ones[a][b]=head_ones[b][a]=pc(diff&low);
   tail_ones[a][b]=tail_ones[b][a]=pc(diff&~low);
  }
 }
} metric;
struct History {
 array<vector<Client>,P> members;
 array<U,P> holes{},strict;
 array<int,P> cached{};
 History(){strict.fill(~low);}
 static int price(U gaps,bool primary){
  int h=pc(gaps&low);
  return weighted(gaps,primary?0:1)+5*(h>0)+12*(primary&&gaps)+2*max(0,h-2)*max(0,h-2);
 }
 int with(int c,const Client* one=nullptr,const Client* two=nullptr)const{
  U h=holes[c],s=strict[c];
  for(const Client* x:{one,two})if(x){U diff=A[c]^A[x->product];h|=diff&x->differences;s&=~diff;}
  if(pc(s&~low)<4)return infinity;
  const bool active=!members[c].empty()||one||two;
  int value=price(h,true)+(active?24:0);
  for(auto x:members[c])value+=price(h&x.differences,false);
  for(const Client* x:{one,two})if(x)value+=price(h&x->differences,false);
  // 現在は共通紙が覆う異色セルも、その紙が短い場合には土台の穴になる。
  U atRisk=~s&~low&~h;
  value+=hazardCost(atRisk);
  if(hazardSetting/100%10){
   for(auto x:members[c])value+=hazardCost(atRisk&x.differences);
   for(const Client* x:{one,two})if(x)value+=hazardCost(atRisk&x->differences);
  }
  return value;
 }
 void rebuild(int c){
  holes[c]=0;strict[c]=~low;
  for(auto x:members[c]){U diff=A[c]^A[x.product];holes[c]|=diff&x.differences;strict[c]&=~diff;}
  cached[c]=with(c);assert(cached[c]<infinity);
 }
 void add(int c,Client x){members[c].push_back(x);rebuild(c);}
 Client remove(int c,int product){
  auto it=find_if(members[c].begin(),members[c].end(),[&](auto x){return x.product==product;});
  assert(it!=members[c].end());Client x=*it;members[c].erase(it);rebuild(c);return x;
 }
};
struct Option {int c=-1,d=-1,increment=infinity;};
struct TwoBest {
 array<pair<int,int>,2> best{{{infinity,INT_MAX},{infinity,INT_MAX}}};
 void add(pair<int,int> value){
  if(value<best[0]){best[1]=best[0];best[0]=value;}
  else if(value<best[1])best[1]=value;
 }
};
AHC_ARCH static Option best_support(const History& history,int a,int b,U differences,const array<bool,P>& paired){
 Client ca{a,differences},cb{b,differences};TwoBest aa,bb;Option result;
 auto accept=[&](int value,int c,int d){
  if(tuple(value,c,d)<tuple(result.increment,result.c,result.d))result={c,d,value};
 };
 const int h_ab=metric.head_ones[a][b],t_ab=metric.tail_ones[a][b],joint=metric.both_prices[a][b];
 const int together_empty=joint+metric.secondary[a][b]+15*(h_ab>0)+12*(differences!=0)+6*max(0,h_ab-2)*max(0,h_ab-2)+24;
 for(int c=0;c<P;++c)if(c!=a&&c!=b&&!paired[c]){
  int x,y,together;
  if(history.members[c].empty()){
   // 未使用土台では2会員の不一致がDを二分し、既存会員への影響は生じない。
   const int h=(int(metric.head_ones[a][c])-int(metric.head_ones[b][c])+h_ab)/2;
   const int w=(int(metric.both_prices[a][c])-int(metric.both_prices[b][c])+joint)/2;
   const int ac=metric.tail_ones[a][c],bc=metric.tail_ones[b][c];
   U wrong=(A[a]^A[c])&differences;
   auto solo=[](int h,int w,bool nonempty){int extra=max(0,h-2);return w+10*(h>0)+12*nonempty+4*extra*extra+24;};
   x=N-H-ac>=4?solo(h,w,wrong!=0):infinity;
   y=N-H-bc>=4?solo(h_ab-h,joint-w,(differences^wrong)!=0):infinity;
   // 尾部で3商品が一致しない各窓は、3つのペア距離の和へちょうど2を加える。
   together=N-H-(t_ab+ac+bc)/2>=4?together_empty:infinity;
const int extra=hazardCost((A[a]^A[c])&~differences&~low);
   if(x<infinity)x+=extra;if(y<infinity)y+=extra;if(together<infinity)together+=extra;
#ifdef VERIFY_EMPTY
   assert(x==history.with(c,&ca)&&y==history.with(c,&cb)&&together==history.with(c,&ca,&cb));
#endif
  }else{
   x=history.with(c,&ca);y=history.with(c,&cb);together=history.with(c,&ca,&cb);
  }
  if(x<infinity)aa.add({x-history.cached[c],c});
  if(y<infinity)bb.add({y-history.cached[c],c});
  if(together<infinity)accept(together-history.cached[c],c,c);
 }
 // 異なる土台の最適解は各商品の最良2個の中にある。全sortとコピーを避ける。
 for(auto [x,c]:aa.best)for(auto [y,d]:bb.best)if(x<infinity&&y<infinity&&c!=d)accept(x+y,c,d);
 return result;
}
struct Coarse {int a,b,shape,jitter,total;U differences;};
struct Selected {Coarse pair;int c,d;};
// The diagonal sentinels exclude the pair's two endpoints while retaining a
// single fixed-width SIMD reduction. Donor products may still be third points.
AHC_ARCH static vector<Coarse> coarse_pairs(const array<bool,P>& paired){
 metric.build();
 alignas(64) static int16_t high[P][P],lower[P][P];
 memcpy(high,metric.secondary,sizeof(high));memcpy(lower,metric.secondary,sizeof(lower));
 for(int a=0;a<P;a++){high[a][a]=4096;lower[a][a]=-4096;}
 vector<Coarse> result;
 for(int a=0;a<P;++a)for(int b=a+1;b<P;++b){
  int jitter=rng(7); // The public random stream is indexed by the full pair table.
  if(paired[a]||paired[b])continue;
  int16_t least=INT16_MAX,most=INT16_MIN;
  for(int c=0;c<P;++c){
   least=min(least,int16_t(high[a][c]-lower[b][c]));
   most=max(most,int16_t(lower[a][c]-high[b][c]));
  }
  const U differences=A[a]^A[b];
  const int weight=metric.secondary[a][b],shape=(weight+4*pc(differences)+4)/8;
  result.push_back({a,b,shape,jitter,shape+jitter+weight+(int(least)-int(most))/2,differences});
 }
 // The stable scatter keeps the incoming (a,b) ordering of costs that tie.
 array<int,1200> offset{};
 for(const auto& e:result)++offset[e.total];
 int sum=0;for(int& c:offset){int old=c;c=sum;sum+=old;}
 vector<Coarse> ranked(result.size());
 for(const auto& e:result)ranked[offset[e.total]++]=e;
 return ranked;
}
#ifdef LOCAL
static uint64_t role_hash(const vector<Selected>& chosen){
 uint64_t value=14695981039346656037ULL;
 for(const auto& x:chosen)for(int id:{x.pair.a,x.pair.b,x.c,x.d}){value^=uint8_t(id);value*=1099511628211ULL;}
 return value;
}
#endif
static void build(unsigned seed){
 H=DonorHead;low=lo(H);rng.x=seed;
 LOCAL_ONLY(joint_role_info={};joint_role_info.seed=seed;trace_stats.count("joint24_attempts"));
 History history;array<bool,P> paired{},reserved{};vector<Selected> chosen;chosen.reserve(64);
 for(const auto& e:patches)for(int a:e.a)paired[a]=true;
 const int count=(P-K+DonorCount)-int(patches.size());
 auto ranked=coarse_pairs(paired);
 array<price_experiment::TwoPartners,P> alternate;
 if(price_experiment::share==4){
  for(const auto& e:ranked){alternate[e.a].add(e.b,e.total);alternate[e.b].add(e.a,e.total);}
  auto fee=[&](const auto& e){return 4*e.total-max(0,alternate[e.a].alternative(e.b,e.total+28)-e.total)
                                   -max(0,alternate[e.b].alternative(e.a,e.total+28)-e.total);};
  stable_sort(ranked.begin(),ranked.end(),[&](const auto& a,const auto& b){return fee(a)<fee(b);});
 }
 for(int stage=0;stage<count;++stage){
  int examined=0,best_score=infinity;Coarse best_pair{};Option best;
  for(const auto& candidate:ranked){
   if(paired[candidate.a]||paired[candidate.b]||reserved[candidate.a]||reserved[candidate.b])continue;
   if(candidate.a==forbiddenPair.first&&candidate.b==forbiddenPair.second)continue;
   auto option=best_support(history,candidate.a,candidate.b,candidate.differences,paired);
   if(option.c<0)continue;
   int score=option.increment+candidate.shape+candidate.jitter;
   if(price_experiment::share==8){
    double cost=observed_sheets::item[candidate.a]+observed_sheets::item[candidate.b];
    score-=clamp(int(lround(4*cost)),-24,24);
   }
   if(best.c<0||tuple(score,candidate.a,candidate.b,option.c,option.d)
                <tuple(best_score,best_pair.a,best_pair.b,best.c,best.d)){
    best_score=score;best_pair=candidate;best=option;
   }
   if(++examined==64)break;
  }
  // 親と同じ設計不成立の例外へ返す。別の構成やseedへここから切り替えない。
  if(best.c<0)throw 1;
  paired[best_pair.a]=paired[best_pair.b]=true;reserved[best.c]=reserved[best.d]=true;
  history.add(best.c,{best_pair.a,best_pair.differences});history.add(best.d,{best_pair.b,best_pair.differences});
  chosen.push_back({best_pair,best.c,best.d});
 }
 LOCAL_ONLY(joint_role_info.pre=role_hash(chosen);joint_role_info.reserved=std::count(reserved.begin(),reserved.end(),true));
 array<int,P> basis{};vector<int> clients,bases;clients.reserve(128);bases.reserve(128);
 for(const auto& x:chosen){basis[x.pair.a]=x.c;basis[x.pair.b]=x.d;}
 for(int a=0;a<P;++a)if(!paired[a])bases.push_back(a);
 for(auto& x:chosen){clients.push_back(x.pair.a);clients.push_back(x.pair.b);}
 sort(clients.begin(),clients.end());
 // 試行seedから再配分を開始する。初回ペア選択で消費した乱数に依存させない。
 rng.x=seed;
 for(int round=0;round<5;++round){
  for(int j=int(clients.size())-1;j>0;--j)swap(clients[j],clients[rng(j+1)]);
  int changes=0;
  for(int a:clients){
   int old=basis[a],selected=-1;Client customer=history.remove(old,a);long long least=LLONG_MAX;
   for(int c:bases){
    int value=history.with(c,&customer),jitter=c==old?0:1+rng(6);
    if(value>=infinity)continue;
    long long key=8LL*(value-history.cached[c])+jitter;
    if(key<least){least=key;selected=c;}
   }
   assert(selected>=0);history.add(selected,customer);basis[a]=selected;changes+=selected!=old;
  }
  LOCAL_ONLY(trace_stats.count_by("joint24_refinement_moves",changes));
  if(!changes)break;
 }
 for(auto& x:chosen){
  x.c=basis[x.pair.a];x.d=basis[x.pair.b];
  assert(!paired[x.c]&&!paired[x.d]);
  patches.push_back({{x.pair.a,x.pair.b},{x.c,x.d},~x.pair.differences});
 }
 LOCAL_ONLY(joint_role_info.post=role_hash(chosen);trace_stats.count("joint24_constructed");trace_stats.count_by("joint24_assignments",128));
}
}
// V6064_JOINT_BACKING_END
int costMatrix[K][K];
// Hungarian法の増加路探索。最短距離を累積距離として保持し、双対変数は
// 増加路発見後に確定する。等しい距離なら元の実装と同じ最小列を選ぶ。
static int relaxScalar(const int* row,const int* v,int shift,int prev,int m,const int* used,
                       int* distance,int* parent) {
 int best=INT_MAX,at=0;
 for(int j=1;j<=m;j++)if(!used[j]){
  int d=row[j-1]-v[j]+shift;
  if(d<distance[j]){distance[j]=d;parent[j]=prev;}
  if(distance[j]<best){best=distance[j];at=j;}
 }
 return at;
}
#if defined(__x86_64__) && defined(__GNUC__)
__attribute__((target("avx2"),always_inline)) static inline __m256i readInt8(const int* p){
 return _mm256_loadu_si256(reinterpret_cast<const __m256i*>(p));
}
__attribute__((target("avx2"),always_inline)) static inline void writeInt8(int* p,__m256i x){
 _mm256_storeu_si256(reinterpret_cast<__m256i*>(p),x);
}
__attribute__((target("avx2")))
static int relaxAvx2(const int* row,const int* v,int shift,int prev,int m,const int* used,
                     int* distance,int* parent) {
 const auto bias=_mm256_set1_epi32(shift),pre=_mm256_set1_epi32(prev);
 auto costs=_mm256_set1_epi32(INT_MAX),indices=_mm256_setzero_si256();
 auto columns=_mm256_setr_epi32(1,2,3,4,5,6,7,8);int j=1;
 for(;j+7<=m;j+=8){
  auto active=readInt8(used+j),old=readInt8(distance+j);
  auto d=_mm256_add_epi32(_mm256_sub_epi32(readInt8(row+j-1),readInt8(v+j)),bias);
  auto improve=_mm256_andnot_si256(active,_mm256_cmpgt_epi32(old,d));
  auto best=_mm256_blendv_epi8(old,d,improve);
  writeInt8(distance+j,best);writeInt8(parent+j,_mm256_blendv_epi8(readInt8(parent+j),pre,improve));
  auto choose=_mm256_andnot_si256(active,_mm256_cmpgt_epi32(costs,best));
  costs=_mm256_blendv_epi8(costs,best,choose);indices=_mm256_blendv_epi8(indices,columns,choose);
  columns=_mm256_add_epi32(columns,_mm256_set1_epi32(8));
 }
 alignas(32) int values[8],idx[8];_mm256_store_si256(reinterpret_cast<__m256i*>(values),costs);
 _mm256_store_si256(reinterpret_cast<__m256i*>(idx),indices);
 int best=INT_MAX,at=0;
 for(int b=0;b<8;b++)if(values[b]<best||(values[b]==best&&idx[b]&&idx[b]<at))best=values[b],at=idx[b];
 for(;j<=m;j++)if(!used[j]){
  int d=row[j-1]-v[j]+shift;
  if(d<distance[j]){distance[j]=d;parent[j]=prev;}
  if(distance[j]<best){best=distance[j];at=j;}
 }
 return at;
}
#endif
template<class Row> static vector<int> hungarianColumns(int n,int m,Row row) {
 vector<int> u(n+1),v(m+1),p(m+1),parent(m+1),distance(m+1),used(m+1),visited(n+1);
 auto relax=relaxScalar;
#if defined(__x86_64__) && defined(__GNUC__) && !defined(HUNGARIAN_SCALAR)
 if(__builtin_cpu_supports("avx2"))relax=relaxAvx2;
#endif
 for(int i=1;i<=n;i++){
  p[0]=i;int j0=0,count=0;distance[0]=0;
  fill(distance.begin()+1,distance.end(),INT_MAX);fill(used.begin(),used.end(),0);
  do{
   used[j0]=-1;visited[count++]=j0;
   int i0=p[j0];
   j0=relax(row(i0-1),v.data(),distance[j0]-u[i0],j0,m,used.data(),distance.data(),parent.data());
  }while(p[j0]);
  int settled=distance[j0];
  for(int k=0;k<count;k++){
   int j=visited[k],delta=settled-distance[j];u[p[j]]+=delta;v[j]-=delta;
  }
  do{int from=parent[j0];p[j0]=p[from];j0=from;}while(j0);
 }
 return p;
}

array<int,K> assignment(){
 auto p=hungarianColumns(K,K,[](int i){return costMatrix[i];});
 array<int,K>matching;for(int j=1;j<=K;j++)matching[p[j]-1]=j-1;return matching;
}
bool frozenGeometry=false;
// 紙の担当と端点が固定された1回の設計内だけ、残窓費用を再利用できる。
struct DesignCost { U residual; unsigned generation; int cost; };
DesignCost designCost[P][128]{};
unsigned costGeneration=0;
bool pricingFixedDesign=false;
#ifdef LOCAL
int designCacheHits=0;
#endif
U costKeys[P][1024]{};unsigned char costVals[P][1024]{};
#if defined(__AVX512BW__)
struct ReverseRepairPrice {
 unsigned epoch=UINT_MAX;
 alignas(64) uint8_t price[K]{};
 U printed[N][3]{},correct[2][N][3]{};
 void prepare(){
  if(epoch==costGeneration)return;
  epoch=costGeneration;memset(printed,0,sizeof(printed));memset(correct,0,sizeof(correct));
  for(int k=0;k<K;k++){
   price[k]=3+min(12,home[k]%256)/2;
   U bit=U(1)<<(k%64);
   for(U m=cards[k].m;m;m&=m-1){
    int z=ctz(m);printed[z][k/64]|=bit;correct[(cards[k].v>>z)&1][z][k/64]|=bit;
   }
  }
 }
 int eval(U residual,int a){
  prepare();
  const __m512i infinity=_mm512_set1_epi8(-1);
  __m512i cost[3]={infinity,infinity,infinity};
  const __m512i fees[3]={_mm512_load_si512(price),_mm512_load_si512(price+64),_mm512_load_si512(price+128)};
  U seen[3]{};for(int k:core[a])seen[k/64]|=U(1)<<(k%64);
  int best=0;
  // A paper enters at its highest residual ink and can extend leftwards
  // only across successive correctly printed cells. Its fee is paid once.
  for(U m=residual;m;){
   int z=63-countl_zero(m);m^=U(1)<<z;
   __m512i tail=_mm512_set1_epi8(char(best));
   auto* valid=correct[(A[a]>>z)&1][z];
   for(int b=0;b<3;b++){
    auto birth=valid[b]&~seen[b];
    cost[b]=_mm512_mask_mov_epi8(cost[b],birth,_mm512_adds_epu8(tail,fees[b]));
    cost[b]=_mm512_mask_mov_epi8(infinity,valid[b],cost[b]);
    seen[b]|=printed[z][b];
   }
   __m512i v=_mm512_min_epu8(cost[0],_mm512_min_epu8(cost[1],cost[2]));
   v=_mm512_min_epu8(v,_mm512_srli_epi32(v,8));
   v=_mm512_min_epu8(v,_mm512_srli_epi32(v,16));
   best=int(_mm512_reduce_min_epu32(_mm512_and_si512(v,_mm512_set1_epi32(255))));
  }
  return best<=120?best:60+pc(residual);
 }
} reverseRepairPrice;
#else
struct ReverseRepairPrice {
 unsigned epoch=UINT_MAX;
 alignas(64) uint8_t fee[K]{};
 alignas(64) uint8_t printed[N][K]{},correct[2][N][K]{};
 void prepare(){
  if(epoch==costGeneration)return;
  epoch=costGeneration;memset(printed,0,sizeof(printed));memset(correct,0,sizeof(correct));
  for(int k=0;k<K;++k){
   fee[k]=3+min(12,home[k]%256)/2;
   for(U m=cards[k].m;m;m&=m-1){
    int z=ctz(m);printed[z][k]=255;correct[(cards[k].v>>z)&1][z][k]=255;
   }
  }
 }
 int eval(U residual,int a){
  prepare();
  alignas(64) uint8_t cost[K],seen[K]{};
  memset(cost,255,sizeof(cost));for(int k:core[a])seen[k]=255;
  int best=0;
  for(U m=residual;m;){
   int z=63-countl_zero(m);m^=U(1)<<z;auto* valid=correct[(A[a]>>z)&1][z];
   uint8_t suffix=min(128,best),least=255;
   // Any cost above 120 has the same externally visible penalty. Clipping
   // before addition permits unsigned byte SIMD on machines without mask registers.
   for(int k=0;k<K;++k){
    uint8_t fresh=suffix+fee[k];
    uint8_t v=(cost[k]&seen[k])|(fresh&~seen[k])|~valid[k];
    cost[k]=v;seen[k]|=printed[z][k];least=min(least,v);
   }
   best=least;
  }
  return best<=120?best:60+pc(residual);
 }
} reverseRepairPrice;
#endif
AHC_ARCH int staticCost(U R,int a,bool ignore=true){
 if(!R)return 0;
 unsigned slot=((R^(R>>31))*0x9e3779b97f4a7c15ULL)>>54;
 bool cache=frozenGeometry&&!ignore;
 if(cache&&costVals[a][slot]&&costKeys[a][slot]==R)return costVals[a][slot]-1;
 auto& memo=designCost[a][slot%128];
 const bool remember=pricingFixedDesign&&ignore;
 if(remember&&memo.generation==costGeneration&&memo.residual==R){
  LOCAL_ONLY(++designCacheHits);
  return memo.cost;
 }
#ifdef VERIFY_REVERSE_PRICE
 int comparison=-1;
#endif
 auto result=[&](int x){
#ifdef VERIFY_REVERSE_PRICE
  if(comparison>=0&&comparison!=x)abort();
#endif
  if(remember)memo={R,costGeneration,x};
  if(cache){costKeys[a][slot]=R;costVals[a][slot]=x+1;}
  return x;
 };
 if(remember){
  int cost=reverseRepairPrice.eval(R,a);
#ifdef VERIFY_REVERSE_PRICE
  comparison=cost;
#else
  return result(cost);
#endif
 }
 int head[65],next[K],len[K],price[K];fill(head,head+65,-1);int goal=64-__builtin_clzll(R);
 for(int k=0;k<K;k++){
  if(ignore&&(k==core[a][0]||(core[a].size()>1&&k==core[a][1])))continue;
  U m=cards[k].m&R;if(!m)continue;
  int l=64-__builtin_clzll(m);U bad=R&lo(l)&((A[a]^cards[k].v)|~cards[k].m);
  int z=bad?64-__builtin_clzll(bad):0;if(z==l)continue;
  len[k]=l;price[k]=3+min(12,home[k]%256)/2;next[k]=head[z];head[z]=k;
 }
 int best[144];fill(best,best+144,-1);best[0]=0;int reached=-1;
 for(int t=0;t<=120;t++)if(best[t]>reached){
  if(best[t]==goal)return result(t);
  for(int z=reached+1;z<=best[t];z++)for(int k=head[z];k>=0;k=next[k])best[t+price[k]]=max(best[t+price[k]],len[k]);
  reached=best[t];
 }
 return result(60+pc(R));
}
array<int,N> bitMap;
// 各紙へ端点と棚位置を割り当て、残余窓の表示費用で割当を再調整する。
void empiricalSlots(const vector<pair<int,int>>&,const array<int,K>&,const array<U,K>&,const array<U,K>&,const array<int,K>&,const array<vector<int>,K>&);
// The first slot at every tail window, both colours at the last window and
// the selected head-completion slots are required. The remaining slots compete
// for the same paper budget under a marginal congestion cost.
bool keep_rectangle=false;
// The first slot at every tail window, both colours at the last window and
// the selected head-completion slots are required. The remaining slots compete
// for the same paper budget under a marginal congestion cost.
void chooseLengths(array<int,K>& seed,vector<pair<int,int>>& slots,const array<U,K>& allow,const array<U,K>& payload,
                   const array<U,K>& color,const array<bool,K>& chosen,const int* phase){
 const int flex=6,slope=20,reward=45;
 array<int,N> old{},at{};for(auto [z,c]:slots)old[z]++;
 struct Place{int z,c,depth;bool required;};vector<Place> options;
 for(auto [z,c]:slots)if(c<0)options.push_back({z,c,at[z]++,true});
 for(int z=H+DonorGroups;z<N;z++)for(int d=0;d<old[z]+flex;d++)options.push_back({z,(phase[z]+d)%2,d,d<(z==N-1?2:1)});
 int M=options.size();vector<int> cost(K*M),eligible(M);
 array<U,K> permitted;permitted.fill(~0ULL);array<int,K>freq{};
 for(auto&e:patches){int id=core[e.a[0]][0];permitted[id]=e.common;for(int j=0;j<(int)e.a.size();j++){int a=e.a[j],b=core[a][1];permitted[b]&=e.common|~(A[a]^A[e.c[j]]);}}
 int gains[K][N]{};
 for(int a=0;a<P;a++)for(int id:core[a]){
  freq[id]++;int gain=0;bool second=core[a][0]!=id;
  for(int z=H;z<N;z++){
   if((permitted[id]>>z&1)&&!((A[a]^color[id])>>z&1))gain+=second&&(permitted[core[a][0]]>>z&1)?1:2;
   gains[id][z]+=gain;
  }
 }
 for(int i=0;i<K;i++)for(int j=0;j<M;j++){
  auto [z,c,d,required]=options[j];bool ok=c<0?i==-c-1:(!chosen[i]&&(allow[i]>>z&1)&&int(color[i]>>z&1)==c);
  eligible[j]+=ok;
  int q=200*freq[i]*d-reward*gains[i][z]+slope*(d-old[z]);
  cost[i*M+j]=q+(ok?0:1000000)-(required?100000:0);
 }
 auto p=hungarianColumns(K,M,[&](int i){return cost.data()+i*M;});
 int draws=0;slots.clear();for(int j=0;j<M;j++){
  if(!p[j+1]){if(options[j].required)throw 1;continue;}
  if(cost[(p[j+1]-1)*M+j]>500000)throw 1;
  draws+=eligible[j]-1;
  seed[p[j+1]-1]=slots.size();
  slots.push_back({options[j].z,options[j].c});
 }
 // Preserve the seed used by the layout price tie-breaks. The rectangular assignment already certifies feasibility.
 while(draws-->0)rng();
}

struct DonorEdge{int cost,a,b,g,rev,err;};

// 同じ中央符号で担当できる商品ペアを8組まで加え、未拘束のビットだけを補う。
// 候補符号が共有する商品を二重に数えず、頭部以外の色違いを減らす。

struct BuiltFamily {array<U,MaxGroups> code{}; vector<DonorEdge> pairs; };


int geometryEpoch=0,geometryTie=0;

// Each four-bit constraint is a ternary word: wildcard, zero, or one.
// The bitset names all product pairs whose first orientation agrees with it.
struct PairIndex {
 static constexpr int blocks=(P*(P-1)/2+63)/64;
 alignas(64) U accept[8][81][blocks];
 int label[16][16]{};
 void build(const uint32_t* mask,const uint32_t* ink){
  for(int m=0;m<16;m++)for(int v=0;v<16;v++)for(int j=3;j>=0;j--)
   label[m][v]=(j==3?0:label[m][v]*3)+((m>>j&1)?1+(v>>j&1):0);
  for(int b=0;b<blocks;b++)for(int group=0;group<8;group++){
   U zero[4]{},one[4]{};
#if defined(__AVX512F__)
   // Transpose 16 edge constraints at a time. Ink contains no bits outside mask.
   for(int t=0;t<64;t+=16){
    __m512i m=_mm512_load_si512(mask+64*b+t),v=_mm512_load_si512(ink+64*b+t);
    for(int j=0;j<4;j++){
     __m512i bit=_mm512_set1_epi32(uint32_t(1)<<(4*group+j));
     one[j]|=U(_mm512_test_epi32_mask(v,bit))<<t;
     zero[j]|=U(_mm512_test_epi32_mask(_mm512_xor_si512(m,v),bit))<<t;
    }
   }
#else
   for(int i=0;i<64;i++){int k=64*b+i,shift=4*group;
    for(int j=0;j<4;j++)if(mask[k]>>(shift+j)&1){
      auto& bits=(ink[k]>>(shift+j)&1)?one[j]:zero[j];bits|=U(1)<<i;
    }
   }
#endif
   accept[group][0][b]=~U(0);
   for(int j=0,n=1;j<4;j++,n*=3)for(int x=0;x<n;x++){
    U any=accept[group][x][b];
    accept[group][n+x][b]=any&~one[j];accept[group][2*n+x][b]=any&~zero[j];
   }
  }
 }
 template<class Visit>void each(uint32_t fixed,uint32_t word,Visit visit)const{
  const U *forward[8],*backward[8];
  for(int g=0;g<8;g++){
   int m=(fixed>>(g*4))&15,v=(word>>(g*4))&15;
   forward[g]=accept[g][label[m][v]];backward[g]=accept[g][label[m][v^15]];
  }
  for(int b=0;b<blocks;b++){U f=~U(0),r=~U(0);
   for(int g=0;g<8;g++){f&=forward[g][b];r&=backward[g][b];}
   for(U hits=f|r;hits;hits&=hits-1){int i=ctz(hits);
    if(!visit(64*b+i,bool(f>>i&1),bool(r>>i&1)))return;
   }
  }
 }
};

// The incumbent's codewords are projected onto each trial head before constructing new cliques.
int inheritedCount=0;
array<U,MaxGroups> inheritedCodes;
BuiltFamily buildFamilies(){
 const int G=DonorGroups;constexpr int NC=960;const int maxPairs=P-K+DonorCount;
 struct Pair {uint8_t a,b,din,dout;U d;};
 struct Claim{int edge,g,rev,rank;};
 // 修復と共有の価格は、頭部符号と許される商品ペアの列挙が終わった後で初めて使う。
 // 同じ採番の親が作った順番を保ち、追加候補では価格から先だけを計算する。
 struct Prepared {
  vector<Pair> edges;array<U,MaxGroups> words{};
  array<array<vector<Claim>,33>,MaxGroups> relaxed;
#ifdef LOCAL
  int groups=0,inherit=0;U head=0;array<U,P> targets{};array<U,MaxGroups> inherited{};
#endif
 };
 static array<Prepared,6> byParent;
 int slot=geometryTie==0?int(G==6):geometryTie+1;
 auto& saved=byParent[slot];
 if(!price_experiment::repair&&!price_experiment::share){
  saved=Prepared{};
#ifdef LOCAL
  saved.groups=G;saved.inherit=inheritedCount;saved.head=low;
  copy_n(A,P,saved.targets.begin());saved.inherited=inheritedCodes;
  trace_stats.count_by("family_prefix_build",1);
#endif
 struct Candidate {U code;vector<int>edges;};
 static vector<Pair> es;static vector<Candidate> pool;
 static int lastEpoch=-1;
 static int cachedTie=-1;static U cachedHead=0;static array<U,P> cachedTargets{};
 static array<bool,NC> copied{};static vector<pair<int,Candidate>> originals;
 if(lastEpoch!=geometryEpoch){
  const bool reuse=lastEpoch>0 && geometryEpoch>0 && cachedTie==geometryTie && cachedHead==low
                   && equal(cachedTargets.begin(),cachedTargets.end(),A);
  // 交換で変わった符号だけを戻す。残る適合辺の一覧は群数に依存しない。
  if(reuse){for(auto& [id,entry]:originals)pool[id]=std::move(entry);}
  else{es.clear();pool.clear();}
  for(auto& record:originals)copied[record.first]=false;
  originals.clear();lastEpoch=geometryEpoch;
  if(geometryEpoch>0){cachedTie=geometryTie;cachedHead=low;copy_n(A,P,cachedTargets.begin());}
#ifdef DAY_GEOM_AUDIT
  if(reuse)cerr<<"[day] pool_hit "<<geometryTie<<"\n";
#endif
 }
 U head=low;
 if(pool.empty()){
 U seed=0x9e3779b97f4a7c15ULL;
 for(int a=0;a<P;a++)seed=(seed^A[a])*0x94d049bb133111ebULL;
 Rng random;random.x=unsigned(seed^(seed>>32))|1u;
 for(int a=0;a<256;a++)for(int b=0;b<a;b++){
  U d=A[a]^A[b];es.push_back({(uint8_t)a,(uint8_t)b,(uint8_t)popcount(d&head),(uint8_t)popcount(d&~head),d&head});}
 sort(es.begin(),es.end(),[](auto&a,auto&b){return a.dout*8+a.din<b.dout*8+b.din;});
 const int edgeCount=es.size();
 vector<int> seeds(edgeCount);iota(seeds.begin(),seeds.end(),0);
 auto seedCost=[&](int i){return 7*es[i].dout+5*es[i].din;};
 partial_sort(seeds.begin(),seeds.begin()+NC,seeds.end(),[&](int i,int j){
  return pair(seedCost(i),i)<pair(seedCost(j),j);
 });
 // Two oriented product pairs share a head word iff their fixed bits
 // agree. Giving them distinct products adds the other condition for
 // assigning both pairs to a single complementary family.
 struct Vertex {uint32_t mask,word;int a,b,price,rank;};
 constexpr int width=128,poolWidth=256,relevance=2;
 using Bits=unsigned __int128;
 auto first=[](Bits b){return U(b)?ctz(U(b)):64+ctz(U(b>>64));};
 alignas(64) static uint32_t mask[P*(P-1)/2],ink[P*(P-1)/2];
 for(int i=0;i<edgeCount;i++){mask[i]=es[i].d;ink[i]=A[es[i].a]&mask[i];}
 static PairIndex index;index.build(mask,ink);
 pool.reserve(NC);
 for(int c=0;c<NC;c++){
  if(c<inheritedCount){
   uint32_t word=inheritedCodes[c];Candidate q{word,{}};
   index.each(~0U,word,[&](int i,bool,bool){q.edges.push_back(i);return true;});
   pool.push_back(std::move(q));continue;
  }
  auto seed=es[seeds[c]];uint32_t fixed=seed.d,word=A[seed.a]&fixed;
  array<Vertex,poolWidth> v;int n=0;
  index.each(fixed,word,[&](int i,bool same,bool reversed){
   auto e=es[i];if(e.a==seed.a || e.b==seed.a || e.a==seed.b || e.b==seed.b)return true;
   uint32_t unused=mask[i]&~fixed;int rank=8*e.dout+relevance*popcount(unused);
   if(same)v[n++]={unused,ink[i]&unused,e.a,e.b,15-e.dout,rank};
   if(reversed&&n<poolWidth)v[n++]={unused,~ink[i]&unused,e.a,e.b,15-e.dout,rank};
   return n<poolWidth;
  });
  if(n>width){
   nth_element(v.begin(),v.begin()+width,v.begin()+n,[](auto&a,auto&b){return a.rank<b.rank;});n=width;
  }
  // 距離0～32の安定な数え上げで、従来と同じ同点順を保つ。
  array<int,33> offset{};array<Vertex,width> ordered;
  for(int i=0;i<n;++i)++offset[15-v[i].price];
  int total=0;
  for(int& x:offset){int frequency=x;x=total;total+=frequency;}
  for(int i=0;i<n;++i)ordered[offset[15-v[i].price]++]=v[i];
  copy_n(ordered.begin(),n,v.begin());
#if defined(__AVX512F__)
  // One mask test detects demanded zero vs demanded one; the transpose
  // produces the adjacency word used directly by the exact clique search.
  alignas(64) array<uint32_t,width> zeros{},ones{};
  array<Bits,P> products{};array<Bits,width> adj{};
  for(int i=0;i<n;i++){
   zeros[i]=v[i].mask^v[i].word;ones[i]=v[i].word;
   Bits bit=Bits(1)<<i;products[v[i].a]|=bit;products[v[i].b]|=bit;
  }
  for(int i=0;i<n;i++){
   Bits blocked=products[v[i].a]|products[v[i].b];
   auto zi=_mm512_set1_epi32(zeros[i]),oi=_mm512_set1_epi32(ones[i]);
   for(int j=0;j<n;j+=16){
    auto z=_mm512_load_si512(zeros.data()+j),o=_mm512_load_si512(ones.data()+j);
    uint16_t conflict=_mm512_test_epi32_mask(o,zi)|_mm512_test_epi32_mask(z,oi);
    blocked|=Bits(conflict)<<j;
   }
   adj[i]=~blocked;
  }
#else
  array<Bits,32> zero{},one{};array<Bits,P> products{};
  for(int i=0;i<n;i++){
   auto a=v[i];Bits bit=Bits(1)<<i;
   for(uint32_t m=a.word;m;m&=m-1)one[countr_zero(m)]|=bit;
   for(uint32_t m=a.mask^a.word;m;m&=m-1)zero[countr_zero(m)]|=bit;
   products[a.a]|=bit;products[a.b]|=bit;
  }
  array<Bits,width> adj{};
  for(int i=0;i<n;i++){
   auto a=v[i];Bits blocked=products[a.a]|products[a.b];
   for(uint32_t m=a.word;m;m&=m-1)blocked|=zero[countr_zero(m)];
   for(uint32_t m=a.mask^a.word;m;m&=m-1)blocked|=one[countr_zero(m)];
   adj[i]=~blocked;
  }
#endif
  int best=0;uint32_t finish=0,cover=0;
  auto search=[&](auto&& self,Bits avail,int score,uint32_t use,uint32_t ink)->void {
   if(score>best){best=score;cover=use;finish=ink;}
   while(avail){
    int j=first(avail);avail&=avail-1;
    if(score+v[j].price*(1+popcount(U(avail))+popcount(U(avail>>64)))<=best)break;
    self(self,avail&adj[j],score+v[j].price,use|v[j].mask,ink|v[j].word);
   }
  };
  search(search,n==128?~Bits(0):(Bits(1)<<n)-1,0,0,0);
  fixed|=cover;word|=finish|(~fixed&random());Candidate q{word,{}};
  index.each(~0U,word,[&](int i,bool,bool){q.edges.push_back(i);return true;});
  pool.push_back(std::move(q));
 }
 // 上位の符号を一部入れ替え、同じ公開表に対して異なる紙の割当も比較する。
 }
 {
  auto add=[&](const vector<int>&v,const vector<int>&w){vector<int>r;r.reserve(v.size()+w.size());set_union(v.begin(),v.end(),w.begin(),w.end(),back_inserter(r));return r;};
 // A complementary assignment replaces an ordinary pair. Charge its tail
 // disagreements against the observed marginal cost of that alternative.
 auto match=[&](const vector<int>& lhs,const vector<int>& rhs={},vector<int>* out=nullptr){
  bool used[P]{};int count=0,value=0;size_t i=0,j=0;
  while((i<lhs.size()||j<rhs.size())&&count<maxPairs){
   int id=(j>=rhs.size()||(i<lhs.size()&&lhs[i]<rhs[j]))?lhs[i++]:rhs[j++];
   const auto& edge=es[id];const int gain=18-edge.dout;
   if(gain<=0||used[edge.a]||used[edge.b])continue;
   used[edge.a]=used[edge.b]=true;++count;value+=gain;
   if(out)out->push_back(id);
  }
  return value;
 };
 // Transpose the codeword-edge incidence list. A common edge runs over
 // all candidates contiguously; each extra edge visits only its posting list.
 // The outer iteration uses the exact edge order of the scalar greedy match.
 vector<vector<uint16_t>> clients(es.size());
 for(int i=0;i<NC;i++)for(int id:pool[i].edges)if(es[id].dout<18)clients[id].push_back(i);
 struct Posting{int id;uint8_t a,b,gain;};
 vector<Posting> postings;
 for(int id=0;id<int(es.size());id++)if(!clients[id].empty()){
  auto e=es[id];postings.push_back({id,e.a,e.b,uint8_t(18-e.dout)});
 }
 alignas(64) static uint8_t occupied[P][NC];
 alignas(64) array<uint16_t,NC> value;
 auto matchPool=[&](const vector<int>& base){
  alignas(64) array<uint8_t,NC> count{};
  memset(occupied,0,sizeof(occupied));value.fill(0);
  size_t j=0;
  for(auto p:postings){
   while(j<base.size()&&base[j]<p.id)++j;
   uint8_t* a=occupied[p.a];uint8_t* b=occupied[p.b];
   if(j<base.size()&&base[j]==p.id){
    for(int i=0;i<NC;i++){
     auto take=uint8_t((a[i]|b[i])==0&&count[i]<maxPairs);
     a[i]|=take;b[i]|=take;count[i]+=take;value[i]+=take*p.gain;
    }
   }else{
    for(int i:clients[p.id])if(!(a[i]|b[i])&&count[i]<maxPairs){
     a[i]=b[i]=1;++count[i];value[i]+=p.gain;
    }
   }
  }
 };
 vector<int> sel,all;bool chosen[NC]{};
 for(int g=0;g<G;g++){
  int best=-10000000,idx=-1;matchPool(all);
  for(int i=0;i<NC;i++)if(!chosen[i]){
   int q=value[i];if(q>best){best=q;idx=i;}
  }
  chosen[idx]=1;sel.push_back(idx);all=add(all,pool[idx].edges);
 }
 for(int g=0;g<G;g++){
  vector<int> base;for(int j=0;j<G;j++)if(j!=g)base=add(base,pool[sel[j]].edges);
  int best=match(all),idx=sel[g];matchPool(base);
  for(int i=0;i<NC;i++)if(!chosen[i]){
   int q=value[i];if(q>best){best=q;idx=i;}
  }
  if(idx!=sel[g]){chosen[sel[g]]=0;chosen[idx]=1;sel[g]=idx;all=add(base,pool[idx].edges);}
 }

 for(int it=0;it<4*G;it++){
  int g=it%G,ix=sel[g];U current=pool[ix].code;vector<int>base;for(int j=0;j<G;j++)if(j!=g)base=add(base,pool[sel[j]].edges);
  vector<int>fix[64];for(int i=0;i<(int)es.size();i++){auto&e=es[i];if(e.dout>=18)continue;U b=e.d&(A[e.a]^current),c=e.d^b;
   if(popcount(b)==1)fix[countr_zero(b)].push_back(i);if(popcount(c)==1)fix[countr_zero(c)].push_back(i);
  }
  int best=match(all),bz=-1;vector<int>bg;for(int z=0;z<H;z++){U bit=1ull<<z;vector<int>keep;for(int e:pool[ix].edges)if(es[e].dout<18&&!(es[e].d&bit))keep.push_back(e);
   auto ge=add(keep,fix[z]);int q=match(base,ge);
   if(q>best){best=q;bz=z;bg=std::move(ge);}
  }
  if(bz>=0){
   if(geometryEpoch>0&&!copied[ix]){originals.emplace_back(ix,pool[ix]);copied[ix]=true;}
   pool[ix].code^=1ull<<bz;pool[ix].edges=std::move(bg);all=add(base,pool[ix].edges);}
 }

  // Each family can exchange one guaranteed head coordinate for additional
  // pairs with a cleaner common tail. A conflicting head bit is left blank
  // on both supplements. Charge all affected clients, including exact edges.
  auto& relaxed=saved.relaxed;
  for(int g=0;g<G;++g)for(int i=0;i<int(es.size());++i){
   auto e=es[i];if(e.dout>17)continue;
   U a=e.d&(A[e.a]^pool[sel[g]].code),b=e.d^a;
   U miss=a;int rev=0;if(pc(b)<pc(a)){miss=b;rev=1;}
   if(pc(miss)>1)continue;
   int z=miss?ctz(miss):32;
   relaxed[g][z].push_back({i,g,rev,4*int(e.dout)+e.din/6});
  }
  for(int g=0;g<G;++g)saved.words[g]=pool[sel[g]].code;
  saved.edges=es;
 }
 }else{
#ifdef LOCAL
  if(saved.groups!=G || saved.inherit!=inheritedCount || saved.head!=low ||
     !equal(saved.targets.begin(),saved.targets.end(),A) || saved.inherited!=inheritedCodes)abort();
  trace_stats.count_by("family_prefix_reused",1);
#endif
 }
 const auto& es=saved.edges;const auto& relaxed=saved.relaxed;
  vector<observed_sheets::Quote> material(es.size());
  vector<price_experiment::RepairQuality> quality;
  if(price_experiment::repair)quality.resize(es.size());
  auto erasurePrice=[&](int id,int z){
   if(price_experiment::repair>=5){auto e=es[id];auto& q=material[id];q.prepare(e.a,e.b);return q.erasure(z,price_experiment::repair);}
   if(!price_experiment::repair)return 7;
   const auto& edge=es[id];auto& q=quality[id];q.prepare(edge.a,edge.b);return q.price(z);
  };
  array<int,MaxGroups> off;off.fill(32);
  vector<Claim> winners;
  // Each erasure option of one complementary family has the same candidates
  // whichever options its siblings currently use. Only the sharing price
  // depends on the combined set of partners.
  array<array<vector<Claim>,33>,MaxGroups> menus;
  array<array<bool,33>,MaxGroups> ready{};
  vector<U> sorted;
  auto score=[&](bool save){
   array<price_experiment::TwoPartners,P> partners;
   for(int g=0;g<G;++g){
    int z=off[g];auto& menu=menus[g][z];
    if(!ready[g][z]){
     ready[g][z]=true;U bit=z<32?U(1)<<z:0;
     auto append=[&](Claim c,bool erased){
      if(erased)c.rank+=erasurePrice(c.edge,z);
      if(price_experiment::share>=5){
       auto e=es[c.edge];auto& q=material[c.edge];q.prepare(e.a,e.b);
       c.rank+=q.pair_charge(e.a,e.b,price_experiment::share);
      }
      if(c.rank<74)menu.push_back(c);
     };
     for(const auto& c:relaxed[g][32])append(c,es[c.edge].d&bit);
     if(bit)for(const auto& c:relaxed[g][z])append(c,true);
    }
    if(price_experiment::share)for(const auto& c:menu){
     auto e=es[c.edge];partners[e.a].add(e.b,c.rank);partners[e.b].add(e.a,c.rank);
    }
   }
   sorted.clear();
   const int weight=price_experiment::share==1?2:price_experiment::share==2?5:3;
   for(int g=0;g<G;++g)for(const auto& c:menus[g][off[g]]){
    int priority=c.rank;
    if(price_experiment::share){
     auto e=es[c.edge];
     int scarcity=max(0,partners[e.a].alternative(e.b,74)-c.rank)+max(0,partners[e.b].alternative(e.a,74)-c.rank);
     priority=8*c.rank-weight*scarcity;
    }
    // Every (edge,family) is unique. Trailing fields permit reconstruction
    // and never participate in a tie between two conflicting claims.
    U key=(U(priority+8192)<<32)|(U(c.edge)<<16)|(U(c.g)<<11)|(U(c.rev)<<10)|U(c.rank+128);
    sorted.push_back(key);
   }
   sort(sorted.begin(),sorted.end());
   bool used[P]{};int n=0,value=0;
   for(U key:sorted){
    int edge=int((key>>16)&65535);const auto& e=es[edge];
    if(used[e.a]||used[e.b])continue;
    int rank=int(key&1023)-128;used[e.a]=used[e.b]=true;value+=74-rank;
    if(save)winners.push_back({edge,int((key>>11)&7),int((key>>10)&1),rank});
    if(++n==maxPairs)break;
   }
   return value;
  };
  int current=score(false);
  for(int step=0;step<((price_experiment::repair==2||price_experiment::repair==8)?4:price_experiment::repair==4?DonorGroups:2);++step){
   int gb=-1,zb=-1,top=current;
   for(int g=0;g<G;++g)if(off[g]==32)for(int z=0;z<32;++z){if(relaxed[g][z].empty())continue;
    off[g]=z;int v=score(false);off[g]=32;if(v>top+3){top=v;gb=g;zb=z;}
   }
   if(gb<0)break;off[gb]=zb;current=top;
  }
  score(true);
  if(price_experiment::share==3||price_experiment::share==4){
   // Fix the chosen masks, then exchange two chosen product pairs if both
   // replacement pairs are admissible and reduce their total repair cost.
   vector<Claim> edgeByPair(P*P,Claim{-1,-1,0,100000});
   for(int g=0;g<G;++g){
    int z=off[g];
    auto registerEdge=[&](Claim c,int add){
     c.rank+=add;if(c.rank>=74)return;auto& e=es[c.edge];
     auto& old=edgeByPair[int(e.a)*P+e.b];
     if(c.rank<old.rank){old=c;edgeByPair[int(e.b)*P+e.a]=c;}
    };
    for(auto c:relaxed[g][32])registerEdge(c,z<32&&(es[c.edge].d>>z&1)?erasurePrice(c.edge,z):0);
    if(z<32)for(auto c:relaxed[g][z])registerEdge(c,erasurePrice(c.edge,z));
   }
   for(int pass=0;pass<3;++pass){
    bool changed=false;
    for(int i=0;i<int(winners.size());i++)for(int j=0;j<i;j++){
     auto e=es[winners[i].edge],v=es[winners[j].edge];
     auto pick=[&](int c,int d){
      const auto& a=edgeByPair[int(e.a)*P+c];const auto& b=edgeByPair[int(e.b)*P+d];
      if(a.edge>=0&&b.edge>=0&&a.rank+b.rank<winners[i].rank+winners[j].rank){
       winners[i]=a;winners[j]=b;changed=true;return true;
      }return false;
     };
     if(!pick(v.a,v.b))pick(v.b,v.a);
    }
    if(!changed)break;
   }
  }
  BuiltFamily f;f.code=saved.words;
  for(auto c:winners){auto e=es[c.edge];f.pairs.push_back({0,e.a,e.b,c.g,c.rev,0});}
  return f;
}

constexpr int tail_mix=100;

// 各ビットは今の試行で確定した相補ペアを指す。取っ手までの順とペアの担当は保つ。
void orderComplementTail(array<int,N>& order){
 if(tail_mix==0||patches.empty())return;
 using W=unsigned __int128;auto cnt=[](W m){return pc(U(m))+pc(U(m>>64));};
 const int first=H+DonorGroups;
 array<W,N> conflicts{};
 for(size_t i=0;i<patches.size();++i){
  const auto& pair=patches[i];U different=A[pair.a[0]]^A[pair.a[1]];
  while(different){const int z=ctz(different);different&=different-1;conflicts[z]|=W(1)<<i;}
 }
 const auto original=order;
 vector<int> remaining(order.begin()+first,order.end());
 W none=(W(1)<<patches.size())-1,once=0,twice=0;
 for(int dest=first;dest<N;++dest){
  const int size=int(remaining.size());
  array<int,N> cost{},rank{},index{};
  for(int i=0;i<size;++i){
   W m=conflicts[remaining[i]];
   cost[i]=cnt(m)+7*cnt(none&m)+3*cnt(once&m)+cnt(twice&m);
   index[i]=i;
  }
  stable_sort(index.begin(),index.begin()+size,[&](int a,int b){return tail_mix>0?cost[a]<cost[b]:cost[b]<cost[a];});
  for(int i=0;i<size;++i)rank[index[i]]=i;
  const int weight=abs(tail_mix);int best=0;
  for(int i=1;i<size;++i)if((100-weight)*i+weight*rank[i]<(100-weight)*best+weight*rank[best])best=i;
  int z=remaining[best];order[dest]=z;remaining.erase(remaining.begin()+best);
  W m=conflicts[z];twice=(twice&~m)|(once&m);once=(once&~m)|(none&m);none&=~m;
 }
#ifdef LOCAL
 assert(equal(original.begin(),original.begin()+first,order.begin()));
 auto firstSum=[&](const array<int,N>& seq){
  W clean=(W(1)<<patches.size())-1;int total=0;
  for(int i=first;i<N;++i){clean&=~conflicts[seq[i]];total+=cnt(clean);}
  return total;
 };
 int moved=0;for(int i=first;i<N;++i)moved+=original[i]!=order[i];
 trace_stats.count_by("tail_moved_windows",moved);
 trace_stats.count_by("tail_first_before",firstSum(original));
 trace_stats.count_by("tail_first_after",firstSum(order));
 trace_stats.count_by("tail_order_pairs",patches.size());
#endif
}

void makeHybridFamilies(){
 H=DonorHead;low=lo(H);
 auto family=buildFamilies();
 auto& chosen=family.pairs;auto& bestCode=family.code;
 LOCAL_ONLY(trace_stats.count_by("marginal_pairs",chosen.size()));
 // 各相補組に異なる取っ手を与える。担当商品の色制約が最も少ない列を選ぶ。
 array<int,MaxGroups> handle{};
 int vote[MaxGroups][64][2]={};
 for(auto e:chosen){U d=(A[e.a]^A[e.b])&~low,c=A[e.rev?e.b:e.a];
  while(d){int z=ctz(d);d&=d-1;vote[e.g][z][c>>z&1]++;}
 }
 // 担当組以外の取っ手で残る不一致を最小化する。担当ペアが落ちる費用を別に加える。
 constexpr int drop=100,early=3;
 int prices[MaxGroups][64]{},total[64]{};
 for(int z=H;z<64;z++)for(int g=0;g<DonorGroups;g++)total[z]+=vote[g][z][0]+vote[g][z][1];
 for(int g=0;g<DonorGroups;g++)for(int z=H;z<64;z++){
  int a=vote[g][z][0],b=vote[g][z][1];prices[g][z-H]=drop*min(a,b)-4*max(a,b)+early*(total[z]-a-b)+int(rng(4));
 }
 auto assigned=hungarianColumns(DonorGroups,N-H,[&](int g){return prices[g];});
 for(int j=0;j<N-H;j++)if(assigned[j+1])handle[assigned[j+1]-1]=j+H;
 for(int g=0;g<DonorGroups;g++){
  int zc=handle[g];
  if(vote[g][zc][1]>vote[g][zc][0])bestCode[g]|=1ULL<<zc;
  for(int i=0;i<2;i++){
   A[P+2*g+i]=(bestCode[g]^(i?~0ULL:0ULL));
   donorMask[2*g+i]=low|(1ULL<<zc);
  }
 }
 // 取っ手の色制約に反する組だけ、通常土台の割当に戻す。

 patches.clear();
 for(auto e:chosen){
  int h=2*e.g+e.rev,j=h^1;U d=A[e.a]^A[e.b];U mismatch=d&(A[e.a]^A[P+h]);
  if(mismatch>>handle[e.g]&1)continue;
  donorMask[h]&=~(mismatch&low);donorMask[j]&=~(mismatch&low);
  patches.push_back({{e.a,e.b},{P+h,P+j},~d});
 }

 activeDonorPairs=patches.size();
 array<int,N> order{};iota(order.begin(),order.end(),0);
 stable_sort(order.begin()+H,order.end(),[&](int a,int b){
   auto key=[&](int z){for(int g=0;g<DonorGroups;g++)if(z==handle[g])return g-N;return z;};
   return key(a)<key(b);
 });
 orderComplementTail(order);
 auto originalMap=bitMap;
 for(int j=0;j<N;j++)bitMap[j]=originalMap[order[j]];
 auto remap=[&](U w){U r=0;for(int j=0;j<N;j++)r|=((w>>order[j])&U(1))<<j;return r;};
 for(U& a:A)a=remap(a);
 for(U& m:donorMask)m=remap(m);
 for(auto& e:patches)e.common=~(A[e.a[0]]^A[e.a[1]]);
}

struct RepairShape {
 static constexpr int W=17;
 U bins[2][6]{};
 void init(){
  memset(bins,0,sizeof(bins));
  for(int z=0;z<64;z++){
   int n[2]{};
   for(int k=0;k<K;k++)if(cards[k].m>>z&1)n[cards[k].v>>z&1]++;
   for(int col=0;col<2;col++){
    int b=min(5,int(bit_width(unsigned(max(1,n[col]))))-1);
    bins[col][b]|=U(1)<<z;
   }
  }
 }
 array<int,W> features(U R,U target,int top,int back)const{
  array<int,W> x{};int n=popcount(R);
  x[0]=n;x[1]=max(0,n-6);x[2]=max(0,n-14);x[3]=max(0,n-22);
  for(int i=0;i<5;i++)x[4+i]=popcount(R&((bins[1][i]&target)|(bins[0][i]&~target)));
  int best[2]{},ready=0,wide=0,second=0;
  U reachable=0;
  for(int k=0;k<K;k++){
   if(k==top||k==back)continue;
   U ink=R&cards[k].m,wrong=ink&(cards[k].v^target);
   if(!ink)continue;
   int errors=popcount(wrong),right=popcount(ink)-errors;
   if(errors<=1)best[1]=max(best[1],right);
   if(errors)continue;
   ready++;wide+=right>=3;reachable|=ink;
   if(right>best[0]){second=best[0];best[0]=right;}
   else second=max(second,right);
  }
  x[9]=n-best[0];x[10]=n-best[1];x[11]=min(8,ready);x[12]=min(6,wide);
  x[13]=n-popcount(reachable);x[14]=max(0,n-best[0]-second);
  x[15]=abs(2*popcount(target&R)-n);x[16]=1;return x;
 }
};

void design(){
 unsigned trial_seed;do{trial_seed=rng();}while(!trial_seed);
 // 前試行の可変な紙・担当・棚を使わず、公開表だけから新しい履歴を組み立てる。
 for(auto& ids:core)ids.clear();
 fill_n(cards,K,Card{});fill_n(home,K,0);fill_n(usage,K,0);frozenGeometry=false;
 makeHybridFamilies();
 joint6064::build(trial_seed);
 rng.x=trial_seed;
 array<U,K>allow{},payload{},color{};array<int,P+MaxDonors>cid;cid.fill(-1);bool paired[P]={};
 for(auto&e:patches)for(int a:e.a)paired[a]=true;
 int next=0;for(int a=0;a<P;a++)if(!paired[a]){int id=next++;cid[a]=id;core[a]={id};allow[id]=~0ULL;payload[id]=low;color[id]=A[a];}
 for(int h=0;h<DonorCount;h++){
  int k=K-DonorCount+h;cid[P+h]=k;allow[k]=donorMask[h];payload[k]=donorMask[h]&low;color[k]=A[P+h];
 }
 for(auto&e:patches){
  int x=next++;allow[x]=e.common;payload[x]=e.common&low;color[x]=A[e.a[0]];
  for(int t=0;t<(int)e.a.size();t++){int a=e.a[t],c=e.c[t],u=cid[c];
   if(c<P)allow[u]&=~(A[a]^A[c]);payload[u]&=allow[x]|~(A[a]^A[c]);core[a]={x,u};}
 }
 assert(next==K-DonorCount);
 int phase[N];
 for(int z=0;z<N;z++){int ones=0;for(int i=0;i<P;i++)ones+=int((A[i]>>z)&1);phase[z]=ones==P/2?(z&1):int(ones>P/2);}
 vector<pair<int,int>> slot;int add[64]={};
 array<bool,K>chosen{};
 for(int h=0;h<DonorCount;h++){
  int k=K-DonorCount+h;chosen[k]=true;slot.push_back({H+h/2,-k-1});add[H+h/2]++;
 }
 const int firstTail=H+DonorGroups;
 for(int z=firstTail;z<N;z++)for(int cl=0;cl<(z==N-1?2:1);cl++){slot.push_back({z,(phase[z]+add[z]++)%2});}
 int extra=K-int(slot.size()),power=155;
 for(int t=0;t<extra;t++){int z=firstTail+min(N-firstTail-1,int((N-firstTail)*pow((t+.5)/extra,100./power)));slot.push_back({z,(phase[z]+add[z]++)%2});}
 array<int,K> rect_seed;chooseLengths(rect_seed,slot,allow,payload,color,chosen,phase);
 array<int,K> match;
 array<int,K>depth;int rankz[64]={};for(int t=0;t<K;t++)depth[t]=rankz[slot[t].first]++;
 {
  int reward=layout_prices[0],freq[K]={};array<vector<int>,K> served;
  for(int a=0;a<P;a++)for(int id:core[a])freq[id]++,served[id].push_back(a);
  array<U,K> optimistic;optimistic.fill(~0ULL);
  for(auto&e:patches){int x=core[e.a[0]][0];optimistic[x]=e.common;for(int t=0;t<(int)e.a.size();t++)optimistic[core[e.a[t]][1]]&=e.common|~(A[e.a[t]]^A[e.c[t]]);}
  for(int id=0;id<K;id++){
   int benefits[65]={};
   for(int z=H;z<N;z++){
    int gain=0;U bit=1ULL<<z;
    for(int a:served[id])if(optimistic[id]&bit){
      if(core[a][0]==id)gain+=2;
      else gain+=(optimistic[core[a][0]]&bit)?1:2;
    }
    benefits[z+1]=benefits[z]+gain;
   }
   for(int t=0;t<K;t++){
    auto [z,col]=slot[t];bool allowed=col<0?id==-col-1:(!chosen[id]&&(allow[id]>>z&1)&&int(color[id]>>z&1)==col);
    costMatrix[id][t]=(allowed?0:1000000)+freq[id]*depth[t]*200-benefits[z+1]*reward+int(rng(17));
   }
   usage[id]=freq[id];
  }
  match=keep_rectangle?rect_seed:assignment();
 }
 array<U,K>actual;auto render=[&](){
 for(int i=0;i<K;i++)home[i]=slot[match[i]].first*256+depth[match[i]];
 actual.fill(~0ULL);for(int h=0;h<DonorCount;h++)actual[K-DonorCount+h]=donorMask[h];
 for(auto&e:patches){
  int x=core[e.a[0]][0];actual[x]=allow[x];U coverX=allow[x]&lo(slot[match[x]].first+1);
  for(int t=0;t<(int)e.a.size();t++){int a=e.a[t],c=e.c[t];actual[cid[c]]&=coverX|~(A[a]^A[c]);}
 }
 for(int i=0;i<K;i++){
  int z=slot[match[i]].first;U m=payload[i]|(actual[i]&lo(z+1)&~low);
  cards[i]={m,color[i]&m,z};
 }
 };render();
 ++costGeneration;pricingFixedDesign=true;
 const int coef=layout_prices[1];
 // 先頭取得後の残窓費を、各紙・端点の既存割当費に加える。
 RepairShape shape;shape.init();
 static constexpr float mu[2][17]={{19.7044811f,14.2972889f,8.35104084f,3.44110107f,0.0f,0.0f,0.976684511f,1.06168056f,2.43044925f,11.2222824f,10.7303152f,3.31804919f,1.38595998f,11.0035219f,10.1521463f,1.59146869f,1.0f},{12.094945f,6.94106913f,1.99043489f,0.179659948f,0.0f,0.0f,0.933056116f,1.0003314f,2.27271128f,7.75459051f,6.10108566f,6.67469692f,2.51376677f,7.08074856f,5.2162962f,2.03938651f,1.0f}};
 static constexpr float inv[2][17]={{0.0914539024f,0.0998301059f,0.134344116f,0.221142262f,10.0f,10.0f,3.13804007f,1.40212667f,0.677803755f,0.132591501f,0.130089238f,0.323579103f,0.887410164f,0.131779626f,0.125271812f,0.652995527f,10.0f},{0.142962247f,0.171978742f,0.306616932f,1.41151774f,10.0f,10.0f,2.70319533f,1.36904633f,0.654817164f,0.156946272f,0.180727139f,0.448050261f,0.504898608f,0.161249787f,0.167331502f,0.549760222f,10.0f}};
 static constexpr float w[2][12][17]={{{-0.989830911f,-0.171650559f,-0.0734083354f,0.315053135f,-0.156478584f,0.0673395544f,0.0102327149f,-0.0171632227f,-0.0396788344f,0.0573532581f,-0.0151979439f,0.0230712593f,-0.00120369764f,-0.156536087f,0.161392808f,0.000492362247f,-0.194288492f},{-0.09915144f,0.260552645f,-0.0921370462f,0.155068934f,-0.199627966f,-0.100794114f,-0.170167774f,-0.110621214f,-0.114858232f,0.0742809922f,0.146365643f,-0.110130772f,0.185593367f,-0.0746609047f,0.149236873f,0.114152089f,-0.228544518f},{0.0953654796f,0.0421651416f,0.125420421f,0.164111048f,0.226416931f,0.188008144f,-0.013138582f,-0.0244488195f,0.00981272571f,0.17108947f,0.155012235f,-0.159821123f,0.144427568f,0.310400188f,0.109911211f,-0.131462574f,0.0592029803f},{0.046635922f,0.253633499f,0.10065724f,-0.042820964f,-0.0646666214f,0.233204424f,0.1172176f,0.0964401811f,0.119815275f,0.317838401f,0.259523273f,0.117032818f,0.0972392485f,0.0585531257f,0.0687307566f,0.0222064834f,-0.031253919f},{0.363873512f,0.203433514f,0.275122285f,-0.0828691944f,-0.117965192f,-0.16483888f,0.0288123265f,-0.043592643f,-0.0481814258f,-0.385392487f,-0.0285818465f,-0.148387298f,-0.141851664f,-0.0651779473f,-0.00495502958f,0.0445018895f,-0.0590303726f},{-0.478040695f,-0.00346535142f,-0.207487047f,0.0577382334f,0.1907278f,-0.211441562f,0.0331785791f,-0.0294809751f,-0.0186530985f,0.162992999f,0.115179166f,0.00169135677f,0.289459974f,0.0721841902f,0.0492065772f,0.0567581169f,0.230623171f},{-0.226584092f,-0.176874891f,-0.0205611531f,-0.0532773435f,0.220865667f,0.216524139f,0.0165505223f,0.117510431f,0.0221944563f,0.0340055861f,-0.00551392045f,-0.0311643239f,-0.0412150137f,-0.364371002f,0.0416908562f,0.198558331f,-0.04759885f},{-0.402558267f,-0.0347223617f,-0.355542302f,-0.13287513f,0.0476702936f,0.12340612f,0.273177058f,0.162837401f,0.200042307f,-0.0782224238f,-0.0272718687f,0.0805877969f,-0.143759489f,-0.144096151f,-0.0710669756f,-0.175822034f,0.237712055f},{-0.355939537f,0.20813787f,-0.0382852256f,0.181393549f,0.169109896f,-0.0522670746f,0.0368125886f,0.108715579f,0.120049722f,0.191535994f,0.103327803f,0.137623072f,0.0285373814f,-0.216680437f,-0.218753919f,-0.0299262032f,-0.0164497234f},{-0.313965619f,-0.113520466f,-0.193215147f,0.0849278718f,0.236739039f,0.187359154f,-0.149431944f,-0.116239868f,-0.0507321171f,-0.324980855f,-0.125374973f,-0.111890964f,-0.161428168f,-0.0895523354f,-0.205599993f,-0.0464585274f,0.230803654f},{-0.258253843f,-0.0978190601f,0.338059932f,-0.108904637f,-0.178968132f,0.130759522f,-0.00897055957f,-0.0557576753f,-0.141990274f,-0.29333663f,-0.0551978424f,-0.19025737f,-0.0529687963f,-0.159964263f,0.0228189658f,-0.0163622294f,-0.107717969f},{-0.177469417f,0.092032291f,0.0159394369f,-0.301379472f,0.0500642173f,0.0590182021f,-0.012786258f,-0.117581114f,-0.150235012f,0.0730808824f,0.0662735626f,-0.0881735235f,-0.0231736284f,0.248247743f,0.196728781f,0.0501863658f,-0.161694214f}},{{-1.14504588f,0.424724489f,-0.0374973267f,-0.0204606354f,-0.0794643611f,0.23095265f,0.00629852852f,0.0601328574f,0.108562954f,0.150118306f,0.0522989184f,-0.0232238807f,0.0807992518f,-0.225696385f,0.00193201378f,0.0437580086f,-0.155303001f},{0.0546180345f,0.10060814f,-0.130015761f,-0.199788496f,-0.170783073f,0.0936852023f,0.152966946f,0.0902541205f,0.0622146018f,-0.0767269731f,0.226179868f,-0.18121928f,-0.0998675823f,-0.0425916277f,0.0517073087f,-0.0179329775f,-0.00419586711f},{-0.17413491f,-0.119335152f,-0.224003822f,-0.276530862f,-0.216965452f,0.108646721f,0.024836529f,-0.0505004786f,-0.198686987f,-0.248335078f,-0.188949972f,0.201809704f,-0.17241393f,-0.219419926f,-0.0661459118f,-0.0186551102f,0.194015339f},{-0.0394563377f,0.188450769f,-0.0929076895f,0.0775990561f,-0.123542033f,0.0844387859f,0.0439229086f,0.0453792661f,0.065935351f,-0.15767926f,0.522162497f,0.0906125531f,0.173474699f,0.311428189f,0.0923690274f,0.0848838165f,0.0233603101f},{-0.258003861f,-0.225677624f,-0.214723304f,-0.272624582f,0.0764086545f,0.12243963f,-0.169313848f,-0.0893420577f,-0.106028929f,-0.353472501f,-0.052129183f,-0.175120428f,-0.255644023f,0.0266116243f,0.0480762906f,-0.142058283f,0.0902899802f},{0.195724845f,-0.189155698f,-0.0818685815f,0.0418065339f,0.0890497789f,0.13349916f,-0.113184012f,-0.0854867324f,0.019003801f,0.265859425f,0.0119836656f,0.0156296715f,-0.108301267f,-0.218448088f,0.246859744f,0.229512617f,0.1770605f},{0.0101066018f,0.0910096392f,-0.236547247f,0.0119130509f,-0.0696380436f,0.202821165f,-0.0373641737f,-0.138051972f,-0.163109869f,-0.12675418f,0.248630196f,-0.0545887165f,0.412632287f,0.205923989f,-0.0295492839f,-0.137902617f,0.00555883115f},{0.343094885f,-0.111596212f,0.102458082f,-0.0361564457f,-0.152546808f,-0.0529264212f,-0.0765379071f,-0.0404328704f,-0.108474545f,0.0746414065f,-0.152633473f,0.21801573f,-0.143625453f,-0.161017373f,0.173842296f,-0.0123175466f,-0.124224424f},{0.0846399218f,0.154928938f,0.187810615f,-0.0934150517f,0.0483458787f,0.150990531f,-0.0385308936f,-0.031140428f,0.110781297f,0.0775099322f,0.135313123f,-0.0947612822f,0.174743414f,0.258220822f,0.229036748f,0.0104522444f,0.0456591547f},{0.242297336f,0.13672246f,0.208845049f,0.0997792259f,0.0164274406f,-0.183611721f,0.0327631645f,-0.0872359797f,-0.172552094f,0.0964120105f,-0.194552869f,-0.139554888f,-0.146548867f,0.177382901f,0.0135491854f,-0.101684175f,-0.122362517f},{0.136946797f,0.0233741663f,-0.000222519826f,-0.0616359748f,-0.0944644809f,0.231243715f,-0.00484038517f,-0.00621764082f,0.00430342183f,-0.664021432f,0.0204901453f,-0.0115440004f,-0.0242339373f,0.538667142f,-0.0450857542f,0.00176908239f,0.0649788827f},{0.259393245f,-0.16018641f,-0.183869451f,-0.0524372049f,0.198990107f,-0.0184373502f,-0.167984203f,-0.252717227f,-0.102012254f,0.0221386068f,0.0230189022f,-0.113384753f,0.0464914106f,0.388625205f,0.1491106f,0.252340347f,-0.113095544f}}};
 static constexpr float b[2][12]={{-0.461613715f,0.164360836f,0.00424634945f,0.146188244f,0.0760359392f,-0.131887868f,0.0113144657f,-0.189511299f,-0.298270881f,-0.13719593f,-0.307009995f,0.0816367269f},{0.156890571f,0.0603344627f,-0.139159307f,-0.105941921f,0.199844226f,-0.020923648f,-0.133570895f,0.366788447f,0.0682372004f,-0.0139749618f,-0.0291254241f,0.181869715f}};
 static constexpr float out[2][12]={{-0.396315604f,0.178230047f,0.181715071f,0.369125485f,-0.245674774f,-0.23907119f,-0.158450142f,-0.107251488f,-0.248086974f,-0.288695633f,-0.249068439f,0.257771134f},{-0.467675805f,0.0701947212f,-0.320347071f,0.246260479f,-0.154908016f,0.122830614f,-0.0659315139f,0.265290111f,0.248652667f,0.194907054f,0.999672115f,-0.110610388f}};
 static constexpr float bias[2]={0.0778129026f,0.0649053156f};
 static constexpr float ymu[2]={5.81052446f,4.2042675f};
 static constexpr float ysd[2]={3.41052508f,2.86064148f};
 auto endpointCost=[&](U residual,int product){
  if(!residual)return 0;
  auto& ids=core[product];int stage=ids.size()==2;
  auto x=shape.features(residual,A[product],ids[0],stage?ids[1]:-1);
  float normalized[17];for(int j=0;j<17;j++)normalized[j]=(x[j]-mu[stage][j])*inv[stage][j];
  float prediction=bias[stage];
  for(int i=0;i<12;i++){
   float h=b[stage][i];for(int j=0;j<17;j++)h+=w[stage][i][j]*normalized[j];
   prediction+=out[stage][i]*max(0.f,h);
  }
  double repair=ymu[stage]+ysd[stage]*prediction;
 #ifdef V5154_PRICE_TRACE
  static int sampled[2]{};
  if(sampled[stage]++<4){cerr<<"[v5154] price "<<stage<<' '<<setprecision(15)<<repair;for(int v:x)cerr<<' '<<v;cerr<<'\n';}
#endif
 // 元費用の全額を残し、凍結したNNが推定する追加手数を同じ単位で足す。
  return int(lround(staticCost(residual,product)+2.0*max(0.,repair)));
 };

 array<vector<int>,K> serving;for(int a=0;a<P;a++)for(int id:core[a])serving[id].push_back(a);
 {
  for(int id=0;id<K;id++){
   int values[64]={};
   for(int a:serving[id]){
    int other=core[a].size()==1?-1:(core[a][0]==id?core[a][1]:core[a][0]);
    U coverOther=other<0?0:cards[other].m&~(cards[other].v^A[a]);
    U prev=~0ULL;int val=0;
    for(int z=max(0,H-1);z<64;z++){
     U mine=(payload[id]|(actual[id]&lo(z+1)&~low))&~(color[id]^A[a]);
     U need=~(mine|coverOther);if(z==H-1||need!=prev){val=endpointCost(need,a);prev=need;}
     values[z]+=val;
    }
   }
   for(int t=0;t<K;t++){
    costMatrix[id][t]+=coef*values[slot[t].first];
   }
  }
  array<int,7> reward[K][N+1]{};
  int gainCoef=layout_prices[2],margin=3;
  if(gainCoef)for(int a=0;a<P;a++){
   U covered=0;for(int k:core[a])covered|=cards[k].m;U R=~covered;if(!R)continue;
   int dest=64-__builtin_clzll(R);int need[K],lens[K],edgecost[K],f[65],rev[65];fill(f,f+65,1000);fill(rev,rev+65,1000);f[0]=0;rev[dest]=0;
   array<vector<int>,65>at;
   for(int id=0;id<K;id++){
    if(id==core[a][0]||(core[a].size()==2&&id==core[a][1]))continue;
    U m=cards[id].m&R;if(!m)continue;int l=64-__builtin_clzll(m);
    U bad=R&lo(l)&((A[a]^cards[id].v)|~cards[id].m);int k=bad?64-__builtin_clzll(bad):0;if(k==l)continue;
    need[id]=k;lens[id]=l;edgecost[id]=3+min(12,home[id]%256)/2;at[l].push_back(id);
   }
   // 到達する端点は0か残窓の位置だけである。その間の窓の前向き費用は常に無限大。
   int rmq[65][65],position[65]{},index[65]{},count=0;
   for(U bits=R;bits;bits&=bits-1){
    int l=ctz(bits)+1;position[++count]=l;index[l]=count;
   }
   for(int t=1;t<=count;++t){
    int l=position[t],best=1000;
    for(int j=t-1;j>=0;--j){
     int k=position[j];best=min(best,f[k]);rmq[l][k]=best;
    }
    for(int id:at[l])f[l]=min(f[l],rmq[l][need[id]]+edgecost[id]);
   }
   if(f[dest]>90)continue;
   for(int t=count;t>0;--t){
    int l=position[t];
    for(int id:at[l])for(int j=index[need[id]];j<t;++j){
     int k=position[j];rev[k]=min(rev[k],edgecost[id]+rev[l]);
    }
   }
   for(int id=0;id<K;id++){
    if(id==core[a][0]||(core[a].size()==2&&id==core[a][1]))continue;
    const U body=payload[id]|(actual[id]&~low),wrong=R&((A[a]^color[id])|~body);
    // Until the next repaired window, both DP endpoints and hence the reward stay fixed.
    for(U hits=body&R;hits;){
     int z=ctz(hits),l=z+1;hits&=hits-1;
     int start=max(z,H-1),end=hits?ctz(hits):N;if(start>=end)continue;
     U bad=wrong&lo(l);int b=bad?64-__builtin_clzll(bad):0;if(b==l)continue;
     int q=margin+f[dest]-rmq[l][b]-rev[l];if(q<=3)continue;
     int bin=min(q,9)-4,excess=max(0,q-9);auto& diff=reward[id];
     ++diff[start][bin];--diff[end][bin];diff[start][6]+=excess;diff[end][6]-=excess;
    }
   }
  }
  // For prices 3..9, decreasing the price by one benefits each active reward
  // above that price once. The last bucket also carries its excess above 9.
  for(int id=0;id<K;id++){
   array<int,7> freq{};
   for(int z=H-1;z<N;z++){
    auto& v=reward[id][z];for(int j=0;j<7;j++)freq[j]+=v[j];
    v[6]=freq[6];int above=0;
    for(int j=5;j>=0;j--){above+=freq[j];v[j]=v[j+1]+above;}
   }
   for(int t=0;t<K;t++)costMatrix[id][t]-=gainCoef*reward[id][slot[t].first][min(12,depth[t])/2];
  }

  // 場合分けの順序を固定する。共通紙を延ばすと、その真下の土台も印字を増やせる。
  // この土台を使うすべての商品について、取引を変えた後のマスクで残窓の費用を差し引く。
  const int closurePrice=19;
  for(const auto& e:patches){
   int x=core[e.a[0]][0];
   int c[2]={core[e.a[0]][1],core[e.a[1]][1]};
   int ns=1+(c[0]!=c[1]);
   U fixed[2]={~U(0),~U(0)},diff[2]={};
   for(const auto& f:patches)for(int t=0;t<(int)f.a.size();t++){
    int a=f.a[t],b=core[a][1],s=b==c[0]?0:(ns==2&&b==c[1]?1:-1);
    if(s<0)continue;
    U delta=A[a]^color[b];
    if(core[a][0]==x)diff[s]|=delta;
    else fixed[s]&=cards[core[a][0]].m|~delta;
   }
   int effect[N]={};
   for(int s=0;s<ns;s++){
    int b=c[s];U body=fixed[s]&lo(cards[b].end+1)&~low;
    for(int a:serving[b]){
     int top=core[a][0];
     U prevOld=~U(0),prevNew=~U(0);int oldCost=0,newCost=0;
     for(int z=H-1;z<N;z++){
      U patchMask=allow[x]&lo(z+1);
      U newBase=payload[b]|(body&(patchMask|~diff[s]));
      U topMask=top==b?0:top==x?patchMask:cards[top].m;
      U correct=~(color[b]^A[a]);
      U rOld=~(topMask|(cards[b].m&correct));
      U rNew=~(topMask|(newBase&correct));
      if(rOld==rNew)continue;
      if(rOld!=prevOld){oldCost=endpointCost(rOld,a);prevOld=rOld;}
      if(rNew!=prevNew){newCost=endpointCost(rNew,a);prevNew=rNew;}
      effect[z]+=newCost-oldCost;
     }
    }
   }
   for(int t=0;t<K;t++)costMatrix[x][t]+=closurePrice*effect[slot[t].first];

  }
  pricingFixedDesign=false;
  empiricalSlots(slot,match,payload,actual,depth,serving);
  const int movePrice=3;
  for(int id=0;id<K;id++)for(int t=0;t<K;t++)costMatrix[id][t]+=movePrice*abs(slot[t].first-slot[match[id]].first);
  match=assignment();render();
 }

}
bool certified(){
 for(int a=0;a<P;a++){
  U cov=0;
  for(int id:core[a]){if((A[a]^cards[id].v)&cards[id].m&~cov)return false;cov|=cards[id].m;}
  for(;;){
   U old=cov;
   for(auto c:cards)if(!((c.v^A[a])&c.m&~cov))cov|=c.m;
   if(cov==~0ULL)break;if(cov==old)return false;
  }
 }
 return true;
}
bool universal=false;
void backupDesign(){
 LOCAL_ONLY(trace_stats.count("backupDesign_calls");trace_stats.mark_fallback());
 universal=true;for(int id=0;id<K;id++){
  int z=id/3;U v=A[rng(P)];if(id%3==0)v&=~(1ULL<<z);if(id%3==1)v|=1ULL<<z;
  cards[id]={lo(z+1),v&lo(z+1),z};home[id]=id;
 }for(auto&v:core)v.clear();
}
void moveCard(State &s,int p,int d){
 if(unsigned(p)>=K||unsigned(d)>=K)abort();
 if(p==d)return;
 int id=s.a[p];s.op.push_back({(uint8_t)id,(uint8_t)d});
 if(p<d)memmove(s.a.data()+p,s.a.data()+p+1,d-p);else memmove(s.a.data()+d+1,s.a.data()+d,p-d);
 s.a[d]=id;
}
int locate(const State& s,int id){
#if defined(__AVX512BW__)
 // Every stack is a permutation of exactly three 64-byte blocks.
 const __m512i needle=_mm512_set1_epi8(char(id));
 for(int i=0;i<K;i+=64){
  const U hits=_mm512_cmpeq_epi8_mask(needle,_mm512_loadu_si512(s.a.data()+i));
  if(hits)return i+ctz(hits);
 }
 return K;
#else
 auto* p=static_cast<const uint8_t*>(memchr(s.a.data(),id,K));
 return p?int(p-s.a.data()):K;
#endif
}
U display(const State&s){
 U cov=0,ans=0;
 for(int id:s.a){ans|=cards[id].v&~cov;cov|=cards[id].m;if(cov==~0ULL)break;}return ans;
}
// 取り除いた遮蔽紙は、対象紙より後ろの隠れた棚へ戻す。
// 現在の積層から、最浅でつかめる窓を他紙が何回塞ぐかを推定する。
// kをyの前から後ろへ通す際に、両紙の期待取得費用の差を加える。
// Choose the canonical shelf and its associated retrieval windows together.
// With a fixed window per request, the number of blockers is a pairwise order cost.
array<int,K> shelfRank;
bool shelfReady=false;
int shelfKey(int k){return shelfReady?shelfRank[k]:home[k];}
void arrangeShelf(State& st){
 constexpr int penalty=3,pairWeight=32,tailWeight=16,passes=10,primaryWeight=32,range=10;
 int original[K],freq[N]{};
 for(int i=0;i<K;i++){original[st.a[i]]=i;U m=cards[i].m;while(m){freq[ctz(m)]++;m&=m-1;}}
 static int skew[K][K];int changed=0;
 for(int round=0;round<passes;round++){
  static uint8_t above[K][N];uint8_t d[N]{};
  for(int k:st.a){memcpy(above[k],d,N);U m=cards[k].m;while(m){++d[ctz(m)];m&=m-1;}}
  int weights[K][N]{};int minDepth[K];
  auto witness=[&](int k,U valid){
   if(!valid)return -1;int zbest=-1,least=INT_MAX;
   while(valid){int z=ctz(valid);valid&=valid-1;int key=int(above[k][z])*256+freq[z];if(key<least){least=key;zbest=z;}}
   return zbest;
  };
  for(int k=0;k<K;k++){
   int z=witness(k,cards[k].m);minDepth[k]=z;weights[k][z]+=32;
   weights[k][cards[k].end]+=tailWeight;
  }
  for(const auto& seq:core)if(!seq.empty()){
   int k=seq[0];if(seq.size()==1){weights[k][minDepth[k]]+=primaryWeight;continue;}
   int y=seq[1],g=witness(k,cards[k].m&~cards[y].m),h=witness(y,cards[y].m&~cards[k].m);
   int op0=h<0?999:above[k][minDepth[k]]+above[y][h];
   int op1=g<0?999:above[y][minDepth[y]]+above[k][g];
   if(op0<=op1&&h>=0){weights[k][minDepth[k]]+=pairWeight;weights[y][h]+=pairWeight;}
   else if(g>=0){weights[y][minDepth[y]]+=pairWeight;weights[k][g]+=pairWeight;}
  }
  for(int k=0;k<K;k++)for(int y=0;y<K;y++)skew[k][y]=penalty*((original[y]>original[k])-(original[y]<original[k]));
  for(int k=0;k<K;k++)for(int z=0;z<N;++z)if(weights[k][z]){
   U bit=U(1)<<z;for(int y=0;y<K;y++)if(cards[y].m&bit){skew[k][y]+=weights[k][z];skew[y][k]-=weights[k][z];}
  }
  int count=0;
  for(int sweep=0;sweep<4;sweep++){
   int n=0;
   for(int t=0;t<K;t++){int k=(sweep+round)%2?t:K-1-t;int old=locate(st,k),least=0,winner=old;
    for(int dir:{-1,1}){int score=0;for(int i=old+dir;unsigned(i)<K&&abs(i-original[k])<=range;i+=dir){
     score+=dir*skew[k][st.a[i]];
     if(score<least){winner=i;least=score;}
    }}
    if(old==winner)continue;n++;++count;
    if(old<winner)move(st.a.begin()+old+1,st.a.begin()+winner+1,st.a.begin()+old);
    else move_backward(st.a.begin()+winner,st.a.begin()+old,st.a.begin()+old+1);
    st.a[winner]=k;
   }
   if(!n)break;
  }
  changed+=count;if(!count)break;
 }
 for(int i=0;i<K;i++)shelfRank[st.a[i]]=i;
 shelfReady=true;LOCAL_ONLY(trace_stats.count_by("shelf_reinsertion_updates",changed));
}

struct GripPrice{
 static constexpr int mix=80,span=4,interval=16,learn=3,any=1;
 struct Neighbor {uint8_t k,y,forward,backward;int8_t orientation;};
 vector<Neighbor> neighbors;
 short change[K][K]{};bool ready=false;
 int use[K]{},primary[K]{},secondary[K]{};
 int price(int k,int y)const{
  if(ready)return change[k][y];
  return (shelfKey(y)>shelfKey(k))-(shelfKey(y)<shelfKey(k));
 }
 void init(){
  // Far endpoints use a constant shelf price throughout the interaction.
  // A nearby pair has two histories, but its two prices are exact negatives.
  neighbors.clear();neighbors.reserve(K*K/5);
  for(int k=0;k<K;k++)for(int y=k+1;y<K;y++){
   int o=(shelfKey(y)>shelfKey(k))-(shelfKey(y)<shelfKey(k));
   change[k][y]=64*o;change[y][k]=-64*o;
   if(abs(cards[k].end-cards[y].end)<=span)neighbors.push_back({uint8_t(k),uint8_t(y),0,0,int8_t(o)});
  }
  for(auto& v:core)if(!v.empty()){
   ++primary[v[0]];
   if(v.size()==2){++secondary[v[1]];}
  }
 }
 void observe(const State& s,int t){
  U covered=0;bool moved[K]{};for(auto m:s.op)moved[m.k]=true;
  for(int k:s.a){
   if(moved[k]&&(cards[k].m&~covered))use[k]+=16;
   covered|=cards[k].m;if(!~covered)break;
  }
  if((t&63)==63)for(int& v:use)v=(v*3+2)/4;
 }
 void update(const State& s,int t){
  if(!mix || (t%interval)!=0)return;
  U planes[8]{};
  auto minimum=[&](U m){for(int b=7;b>=0;b--){U z=m&~planes[b];if(z)m=z;}return m;};
  int demand[K];for(int k=0;k<K;k++)demand[k]=10+3*primary[k]+2*secondary[k]+min(80,learn*use[k]/24);
  int pos[K];U witness[K];
  for(int i=0;i<K;i++){
   int k=s.a[i];pos[k]=i;witness[k]=minimum(cards[k].m);
   U m=cards[k].m;for(int b=0;m;b++){U carry=m&planes[b];planes[b]^=m;m=carry;}
  }
  auto updateHistory=[&](uint8_t h,int k,int y){
   U w=witness[k],m=cards[y].m;
   int cost=32*int(any&&pos[y]<pos[k]?bool(w&m):!(w&~m));
   return uint8_t(ready?(7*h+cost+4)/8:cost);
  };
  for(auto& q:neighbors){
   int k=q.k,y=q.y;q.forward=updateHistory(q.forward,k,y);q.backward=updateHistory(q.backward,y,k);
   int e=clamp(64*int(q.orientation)+mix*(int(q.forward)*demand[k]-int(q.backward)*demand[y])/320,-20000,20000);
   change[k][y]=e;change[y][k]=-e;
  }
  ready=true;
 }
} pricing;

int restoreAt(const State&s,int p,int minimum){
 int best=minimum,start=0,score=0;
 for(int i=minimum+1;i<K;i++){
  score+=pricing.price(s.a[p],s.a[i]);
  if(score<start){start=score;best=i;}
 }
 return best;
}
struct Reach{int cost,z;};
State fetchCache[K];unsigned cacheStamp[K]={},queryStamp=0;

// 対象紙を露出させる費用が最も小さい印字窓を選ぶ。
Reach reach(const State&s,int pos,U eligible){
 U need=cards[s.a[pos]].m&eligible;
 if(!need)return {999,-1};
 U blocked=s.cover,poss=need&~blocked;
 if(!poss)return {999,-1};
 for(int i=s.front;i<pos;i++)blocked|=cards[s.a[i]].m;
 if(U open=need&~blocked)return {0,ctz(open)};
 U cnt[8]={};for(int i=s.front;i<pos;i++){
  U carry=cards[s.a[i]].m&poss;
  for(int j=0;carry;j++){U old=cnt[j];cnt[j]^=carry;carry&=old;}
 }
 int cost=0;for(int j=7;j>=0;j--){U fewer=poss&~cnt[j];if(fewer)poss=fewer;else cost|=1<<j;}
 return {cost,ctz(poss)};
}
bool fetch(State&s,int id,int where,U eligible,bool commit){
 bool shared=queryStamp&&s.front==0&&where==0&&s.op.empty()&&eligible==~0ULL&&!commit;
 if(shared&&cacheStamp[id]==queryStamp){s=fetchCache[id];return true;}
 int pos=locate(s,id);
 Reach rr=reach(s,pos,eligible);
 if(rr.z<0)return false;
 U bit=1ULL<<rr.z;
 for(int i=s.front;i<pos;){
  if(cards[s.a[i]].m&bit){int d=restoreAt(s,i,pos);moveCard(s,i,d);pos--;}
  else i++;
 }
 if(commit){
  bool free=true;for(int i=s.front;i<pos&&free;i++)if(cards[s.a[i]].m&~s.cover)free=false;
  if(free)s.front=pos+1;
  else {moveCard(s,pos,where);s.front=where+1;}
  s.cover|=cards[id].m;
 }else moveCard(s,pos,where);
 if(shared){cacheStamp[id]=queryStamp;fetchCache[id]=s;}
 return true;
}
bool fetchCore(State& s,const vector<int>& ids,bool reverse){
 s.front=0;s.cover=0;
 if(ids.size()==1){
  if(!fetch(s,ids[0],0,~0ULL,false))return false;
 }else if(reverse){
  if(!fetch(s,ids[1],0,~0ULL,false))return false;
  s.front=1;s.cover=cards[ids[1]].m;
  if(!fetch(s,ids[0],0,~0ULL,false))return false;
 }else{
  if(!fetch(s,ids[0],0,~0ULL,false))return false;
  s.front=1;s.cover=cards[ids[0]].m;
  if(!fetch(s,ids[1],1,~0ULL,false))return false;
 }
 s.front=ids.size();s.cover=0;for(int id:ids)s.cover|=cards[id].m;return true;
}
struct Select{int card,z,cost,gain;float key;};
vector<Select> candidates(const State&s,U tgt){
 vector<Select>v;v.reserve(24);if(s.cover==~U(0))return v;
 U counts[8]{},seen=0;int miss=ctz(~s.cover);
 for(int i=s.front;i<K;i++){
  int id=s.a[i];auto c=cards[id];U ex=c.m&~s.cover;
  if(ex&&!(ex&(c.v^tgt))){
   U candidates=ex&~seen;int cost=0;
   if(!candidates){
    candidates=ex;
    for(int b=7;b>=0;b--){U t=candidates&~counts[b];
     if(t)candidates=t;else cost|=1<<b;}
   }
   int gain=pc(ex);float key=float(gain)/(1.15f+cost);if(ex>>miss&1)key+=.7f;
   v.push_back({id,ctz(candidates),cost,gain,key});
  }
  seen|=ex;for(int b=0;ex;b++){U t=counts[b]&ex;counts[b]^=ex;ex=t;}
 }
 sort(v.begin(),v.end(),[](auto&a,auto&b){return a.key>b.key;});return v;
}
struct Sched {
 array<int,K>rank{}, first{},last{};
 int rcount=0;
 vector<int>ord;
 State initial,goal;
 void init(const State& start,const State& end){
  initial=start;initial.op.clear();initial.front=0;initial.cover=0;goal=end;
  first.fill(-1);last.fill(-1);rcount=0;
  for(int i=0;i<K;i++)rank[end.a[i]]=i;
  for(int i=0;i<(int)end.op.size();i++){int id=end.op[i].k;if(first[id]<0){first[id]=i;rcount++;}last[id]=i;}
 }
 array<bool,K> movedSet(int mode){
  array<bool,K>moving;
  for(int i=0;i<K;i++)moving[i]=first[i]>=0;
  if(mode){
   int prev=-1,start=0,lens[K],idx[K],parent[K];
   for(int i=0;i<=K;i++)if(i==K||!moving[initial.a[i]]){
    int end=i==K?K:rank[initial.a[i]];
    int ns=0;
    for(int z=start;z<i;z++){
     int s=rank[initial.a[z]];if(s<=prev||s>=end)continue;
     int q=lower_bound(lens,lens+ns,s)-lens;if(q==ns)ns++;
     lens[q]=s;idx[q]=z;parent[z]=q?idx[q-1]:-1;
    }
    if(ns)for(int j=idx[ns-1];j>=0;j=parent[j])moving[initial.a[j]]=false;
    start=i+1;prev=end;
   }
  }
  return moving;
 }
AHC_ARCH  bool attempt(State& result,int it,int limit,const array<bool,K>& moving,int left) {
  if(left>=limit)return false;
  auto mv=moving;
  array<uint8_t,K> paper, gap, goal_gap, anchors;
  array<U,K+1> prefix{};
  int n=0,nfixed=0;
  for(int j=0;j<K;j++){
   int id=initial.a[j];
   if(mv[id]){paper[n]=id;gap[n++]=nfixed;}
   else{anchors[nfixed]=id;prefix[nfixed+1]=prefix[nfixed]|cards[id].m;++nfixed;}
  }
  // 恒久的に据え置く紙は開始順と目標順が等しい。gapにはその手前の据置枚数を持つ。
  nfixed=0;
  for(int id:goal.a){if(mv[id])goal_gap[id]=nfixed;else ++nfixed;}
  State s=initial;
  while(left){
   int winner=-1,insert=-1,from=-1,dest=-1;float best=-1e20f;
   U prior=0;
   for(int j=0;j<n;j++){
    int a=paper[j];
    bool visible=cards[a].m&~(prior|prefix[gap[j]]);
    prior|=cards[a].m;
    if(!mv[a]||!visible)continue;
    const int g=goal_gap[a],f=gap[j]+j;
    int put=int(upper_bound(gap.begin(),gap.begin()+n,g)-gap.begin());
    for(int k=put-1;k>=0&&gap[k]==g;--k)
     if(!mv[paper[k]]&&rank[paper[k]]>rank[a])put=k;
    int d=g+put;d-=d>f;
    float score=0;
    switch((it/2)%5){
     case 0:score=d-f;break;
     case 1:score=-first[a];break;
     case 2:score=-last[a];break;
     case 3:score=rank[a];break;
     case 4:score=-cards[a].end+float(d-f)*.3;break;
    }
    if(it>=3)score+=int(rng(250))-125;
    if(score>best){best=score;winner=j;insert=put-(put>j);from=f;dest=d;}
   }
   if(winner<0)return false;
   int a=paper[winner];
   if(from!=dest)s.op.push_back({(uint8_t)a,(uint8_t)dest});
   if(winner<insert){
    memmove(paper.data()+winner,paper.data()+winner+1,insert-winner);
    memmove(gap.data()+winner,gap.data()+winner+1,insert-winner);
   }else if(winner>insert){
    memmove(paper.data()+insert+1,paper.data()+insert,winner-insert);
    memmove(gap.data()+insert+1,gap.data()+insert,winner-insert);
   }
   paper[insert]=a;gap[insert]=goal_gap[a];mv[a]=false;--left;
  }
  if(s.op.size()>=unsigned(limit))return false;
  int at=0,fi=0;
  for(int j=0;j<n;j++){
   while(fi<gap[j])s.a[at++]=anchors[fi++];
   s.a[at++]=paper[j];
  }
  while(fi<nfixed)s.a[at++]=anchors[fi++];
  if(s.a!=goal.a)return false;
  result=std::move(s);return true;
 }
 bool attempt(State& result,int it,int limit){
  auto moving=movedSet(it%3!=0);
  return attempt(result,it,limit,moving,count(moving.begin(),moving.end(),true));
 }
 void run(const State&start,State&result,int reps){
  init(start,result);
  int orig=result.op.size();auto lis=movedSet(1);int floor=count(lis.begin(),lis.end(),true);
  if(floor>=orig)return;
  auto all=movedSet(0);
  for(int it=0;it<reps&&(int)result.op.size()>floor;it++){
   State s;const bool reduced=it%3!=0;
   if(attempt(s,it,result.op.size(),reduced?lis:all,reduced?floor:rcount))result=std::move(s);
  }
 }
};
// 積層順の保護した先頭部分は既に正しい窓を表示している。
// 残余窓では各紙の最後の印字までを仮想prefixとし、穴や誤色を含むsuffixをDPで除く。
// 仮想取得に使う窓も、実際に露出した印字窓に限定する。
bool liveTailCache=false;
struct TailDP {
 struct Memo{
  array<uint8_t,K> before,after;
  vector<Move> tail;
  U open=0,target=0;
  int front=0,dist=0;
 };
 array<Memo,16> memo;
 unsigned memoStamp=0;
 int memoSize=0,memoNext=0;
 struct Cached {U R=0,target=0;unsigned char len[K],err[K];};array<Cached,128>cache{};
 struct FullEntry {U target=0;int epoch=0;bool possible=false;};
 array<FullEntry,1024> fullMemo{};
 int epoch=1;
 void resetGeometry(){
  ++epoch;for(auto& e:cache){e.R=0;e.target=0;}
 }
 bool fullPossible(U target){
  auto& e=fullMemo[(target*0x9e3779b97f4a7c15ULL)>>54];
  if(e.epoch!=epoch||e.target!=target){
   // A candidate prefix sheet can advance from covered height h to l only
   // if every hole or wrong color lies below h. Failure of this closure
   // rules out every physical shelf order, even without a move limit.
   int step[N+1]{};
   for(auto c:cards){
    int l=bit_width(c.m);U wrong=lo(l)&(~c.m|(c.v^target));
    int bad=bit_width(wrong);step[bad]=max(step[bad],l);
   }
   int reach=0;for(int h=0;h<=reach&&reach<N;h++)reach=max(reach,step[h]);
   e={target,epoch,reach==N};
  }
  return e.possible;
 }
 int secondary[65][K+1];
 int n,k,le[K],bad[K],pos[K],ct[65];unsigned char counts[65][K+1],cumul[65][K+1],ids[65][K];
 int dist[65][K+1];struct Back{uint8_t h,x,id;};Back prev[65][K+1];int states[65][K+1],ns[65];
 unsigned char wcounts[65][K+1],wcumul[65][K+1];
 template<bool weighted=false> AHC_ARCH bool solve(State&s,U target,bool partial=false,const U*cut=nullptr,int maxCost=10000,const bool* cheap=nullptr){
  U R=~s.cover;n=pc(R);k=K-s.front;if(!n)return true;
  if(!cut&&frozenGeometry&&R==~U(0)&&!fullPossible(target))return false;
  const bool reuse=liveTailCache&&!weighted&&!partial&&!cut;
  array<uint8_t,K> original;int prefixLength=s.op.size();
  if(reuse){
   if(memoStamp!=queryStamp){memoStamp=queryStamp;memoSize=memoNext=0;}
   for(int i=0;i<memoSize;++i){
    const auto& e=memo[i];
    if(e.front!=s.front||e.open!=R||e.target!=target||e.before!=s.a)continue;
    // The caller's prefix can differ for the same physical shelf. The
    // cached distance is the original DP bound before no-op elimination.
    if(e.dist>maxCost)return false;
    s.a=e.after;s.op.insert(s.op.end(),e.tail.begin(),e.tail.end());return true;
   }
   original=s.a;
  }
  fill(ct,ct+n+1,0);
  Cached* model=nullptr;
  if(!cut&&frozenGeometry){
   unsigned key=unsigned(((R^(target*0x94d049bb133111ebULL))*0x9e3779b97f4a7c15ULL)>>57);
   model=&cache[key];if(model->R!=R||model->target!=target){
    model->R=R;model->target=target;
    for(int id=0;id<K;id++){U m=cards[id].m&R;int l=0,b=0;if(m){U top=lo(64-__builtin_clzll(m));l=pc(R&top);U wrong=R&top&((cards[id].v^target)|~cards[id].m);if(wrong)b=pc(R&lo(64-__builtin_clzll(wrong)));}model->len[id]=l;model->err[id]=b;}
   }
  }
  for(int i=0;i<k;i++){
   int id=s.a[s.front+i];U m=cards[id].m&R;int l=0,b=0;
   if(model){l=model->len[id];b=model->err[id];}
   else if(m){U top=lo(64-__builtin_clzll(m));l=pc(R&top);U wrong=R&top&(((cards[id].v^target)&cards[id].m)|(~cards[id].m&(cut?cut[id]:~0ULL)));if(wrong)b=pc(R&lo(64-__builtin_clzll(wrong)));}
   le[id]=l;bad[id]=b;pos[id]=i;ids[l][ct[l]++]=id;
  }
  const bool valueTies=frozenGeometry&&!weighted&&!partial;
  int tieSelected=0;secondary[0][0]=0;
  if constexpr(weighted)memset(wcumul[0],0,k+1);
  memset(cumul[0],0,k+1);ns[0]=1;states[0][0]=0;dist[0][0]=0;
  for(int l=1;l<=n;l++){
   ns[l]=0;fill(dist[l],dist[l]+k+1,10000);
   int at=0;
   for(int q=0;q<ct[l];q++){int e=pos[ids[l][q]]+1;memset(counts[l]+at,q,e-at);at=e;}
   memset(counts[l]+at,ct[l],k+1-at);
   for(int z=0;z<=k;z++)cumul[l][z]=cumul[l-1][z]+counts[l][z];
   if constexpr(weighted){
    int at=0,price=0;
    for(int q=0;q<ct[l];q++){int id=ids[l][q],e=pos[id]+1;memset(wcounts[l]+at,price,e-at);at=e;price+=!cheap[id];}
    memset(wcounts[l]+at,price,k+1-at);
    for(int z=0;z<=k;z++)wcumul[l][z]=wcumul[l-1][z]+wcounts[l][z];
   }
   for(int j=0;j<ct[l];j++){
    int id=ids[l][j],p=pos[id];
    for(int h=bad[id];h<l;h++){
     int mp=cumul[l-1][p]-cumul[h][p];
     for(int q=0;q<ns[h];q++){
      int x=states[h][q],mx=cumul[l-1][x]-cumul[h][x];
      int add=p>=x?mx+j+(mp>mx):mx+counts[l][x];if constexpr(weighted){int wmx=wcumul[l-1][x]-wcumul[h][x];
       add=p>=x?wmx+wcounts[l][p]+((mp>mx)&&!cheap[id]):wmx+wcounts[l][x];}int nx=max(p+1,x),v=dist[h][x]+add;
      if(v<=dist[l][nx]){
       if(valueTies){
        bool active=p<x||(mp>mx&&p>=x);
        // Prefer to keep deep active repair sheets in the stationary shelf.
        int extra=active?40+max(0,shelfRank[id]-s.front):0;
        int tie=secondary[h][x]+extra;
        if(v==dist[l][nx]){if(tie>=secondary[l][nx])continue;++tieSelected;}
        secondary[l][nx]=tie;
       }else if(v==dist[l][nx])continue;
       if(dist[l][nx]==10000)states[l][ns[l]++]=nx;dist[l][nx]=v;prev[l][nx]={(uint8_t)h,(uint8_t)x,(uint8_t)id};}
     }
    }
   }
  }
  int x=min_element(dist[n],dist[n]+k+1)-dist[n];
  if(valueTies){
   if(dist[n][x]<10000){for(int j=x+1;j<=k;j++)if(dist[n][x]==dist[n][j]&&secondary[n][j]+j<secondary[n][x]+x)x=j;}
   LOCAL_ONLY(trace_stats.count_by("dp_value_retie",tieSelected));}
  if(dist[n][x]>=10000||dist[n][x]>maxCost)return false;
  int actualDistance=dist[n][x];
  int l=n;bool moves[K]={},acts[K]={};vector<int> moved,active;
  while(l){
   auto [h,px,id]=prev[l][x];int p=pos[id];
   for(int m=l;m>h;m--){
    int b=m==l?max((int)px,p):int(px);
    for(int t=0;t<counts[m][b];t++){int a=ids[m][t];moves[a]=1;moved.push_back(a);}
   }
   if(p>=px&&cumul[l-1][p]-cumul[h][p]>cumul[l-1][px]-cumul[h][px]){moves[id]=1;moved.push_back(id);}
   acts[id]=true;active.push_back(id);l=h;x=px;
  }
  reverse(active.begin(),active.end());vector<int>fin;int i=0,h=0,base=s.front;
  for(int id:active){
   while(i<k&&(moves[s.a[base+i]]||le[s.a[base+i]]<=h)){if(!moves[s.a[base+i]])fin.push_back(s.a[base+i]);i++;}
   if(!moves[id]){if(i>=k||s.a[base+i]!=id)return false;i++;}
   fin.push_back(id);h=le[id];
  }
  while(i<k){if(!moves[s.a[base+i]])fin.push_back(s.a[base+i]);i++;}
  vector<int>full=fin;
   for(auto it=moved.rbegin();it!=moved.rend();it++)if(!acts[*it]){
    int id=*it,cov=0,delta=0,best=1000000,dest=-1;
    for(int j=0;j<=int(full.size());j++){
     if(cov>=le[id]&&delta<best)best=delta,dest=j;
     if(j<int(full.size())){int v=full[j];cov=max(cov,le[v]);delta+=pricing.price(id,v);}
    }
    if(dest<0)return false;full.insert(full.begin()+dest,id);
   }
  int rank[K];for(int i=0;i<k;i++)rank[full[i]]=i;
  for(size_t move=0;move<moved.size();move++){
   int cover=0,id=-1,from=-1;
   for(int j=base;j<K;j++){int v=s.a[j];if(le[v]>cover){if(moves[v]){id=v;from=j;break;}cover=le[v];}}
   if(id<0)return false;moves[id]=false;
   int j=base;
   while(j<K&&(j==from||moves[s.a[j]]||rank[s.a[j]]<rank[id]))j++;
   if(!acts[id]){
    int far=j;for(int z=j;z<K;z++)if(moves[s.a[z]]&&le[s.a[z]]<=le[id])far=z+1;
    for(int z=j;z<far;z++)if(z!=from&&!moves[s.a[z]])rank[id]=max(rank[id],rank[s.a[z]]);
    j=far;
   }
   int d=j-(j>from);moveCard(s,from,d);
  }
  bool good=partial||display(s)==target;
  if(reuse&&good){
   auto& e=memo[memoNext];memoNext=(memoNext+1)%int(memo.size());memoSize=min(int(memo.size()),memoSize+1);
   e.before=original;e.after=s.a;e.open=R;e.target=target;e.front=s.front;e.dist=actualDistance;
   e.tail.assign(s.op.begin()+prefixLength,s.op.end());
  }
  return good;
 }
} taildp;
bool repairOne(State&s,U tar){
 const int start=s.front;U R=~s.cover,seen=0,twice=0,out=0,backup=0;
 int visible[64],nv=0;
 for(int i=start;i<K&&twice!=R;i++){auto c=cards[s.a[i]];U m=c.m&R,fresh=m&~seen,second=m&seen&~twice;if(fresh){visible[nv++]=i;out|=c.v&fresh;}backup|=c.v&second;twice|=m&seen;seen|=m;}
 U goal=tar&R;if(out==goal)return true;U prior=0;
 for(int q=0;q<nv;q++){
  int p=visible[q],id=s.a[p];U m=cards[id].m&R,expo=m&~prior;prior|=m;
  U base=(out&~expo)|(backup&expo),left=m;
  if((base&~left | (cards[id].v&left))==goal){moveCard(s,p,start);return true;}
  int d=start;
  for(int j=start;j<K;j++)if(j!=p){
   d++;U nleft=left&~cards[s.a[j]].m;if(nleft!=left){
    left=nleft;if(((base&~left)|(cards[id].v&left))==goal){moveCard(s,p,d);return true;}
    if(!left)break;
   }
  }
 }
 return false;
}
void shadowCuts(const State&order,U target,array<U,K>&cut){
 U below=0;
 for(int i=K-1;i>=0;i--){int id=order.a[i];auto c=cards[id];cut[id]=(~c.m)&(below^target);below=(below&~c.m)|c.v;}
}
// 一部の穴制約を緩めてから実積層で再生し、不一致を生んだ仮想表示紙の穴制約を強める。
bool multiHole(const State&base,const State&reference,U target,State&out,int cap,int seed,int rounds){
 array<U,K>cut{};if(seed==1)shadowCuts(reference,target,cut);if(seed==2)shadowCuts(base,target,cut);
 U R=~base.cover;
 for(int it=0;it<rounds;it++){
  State s=base;if(!taildp.solve(s,target,true,cut.data()))return false;
  if(int(s.op.size())>=cap)return false;
  U bad=(display(s)^target)&R;
  if(!bad||(pc(bad)<=3&&int(s.op.size())+1<cap&&repairOne(s,target))){if(display(s)!=target)abort();out=std::move(s);return true;}
  if(it+1<rounds){
   U cov=0;bool changed=false;
   for(int j=base.front;j<K;j++){
    int id=s.a[j];U m=cards[id].m&R;if(!m)continue;
    U projected=R&lo(64-__builtin_clzll(m));U errors=bad&projected&~cov;
    if(errors){cut[id]|=errors;changed=true;}
    cov|=projected;if((bad&~cov)==0)break;
   }
   if(!changed)break;
  }
 }
 return false;
}

bool ghost(const State& start,const vector<int>& ids,U target,State &best,int bound,int tries){
 State end=start;end.op.clear();
 for(int j=ids.size()-1;j>=0;j--){int id=ids[j];moveCard(end,locate(end,id),0);}
 end.front=ids.size();end.cover=0;for(int id:ids)end.cover|=cards[id].m;
 if(!taildp.solve(end,target))return false;
 bool forced[K]={};int count=0;for(auto op:end.op)if(!forced[op.k])forced[op.k]=1,count++;

 for(int i=K-1;i>=0;i--){
  int id=start.a[i];if(!forced[id])continue;U fixed=0;
  for(int j=0;j<i;j++)if(!forced[start.a[j]])fixed|=cards[start.a[j]].m;
  if(cards[id].m&~fixed)continue;
  U bits=cards[id].m;int minv=1000,zbest=-1;
  while(bits){int z=ctz(bits);bits&=bits-1;int n=0;
   for(int j=0;j<i&&n<minv;j++)if(!forced[start.a[j]])n+=(cards[start.a[j]].m>>z)&1;
   if(n<minv)minv=n,zbest=z;}
  U bit=1ULL<<zbest;
  for(int j=0;j<i;j++){int c=start.a[j];if(!forced[c]&&(cards[c].m&bit)){forced[c]=1;end.op.push_back({(uint8_t)c,0});count++;}}
 }

 if(count>=bound)return false;
 Sched sc;
 sc.init(start,end);
 for(int it=0;it<tries;it++){State out;if(sc.attempt(out,it,bound)){
    if(display(out)!=target)abort();best=std::move(out);return true;
 }}
 return false;
}
struct Trimmer {
 struct A{int id,d,bef,aft;};
 vector<A>script;vector<array<uint8_t,K>> prefix;
 static int energy(const State&s){
  array<int,64>count{};int inv=0;
  for(int id:s.a){int e=cards[id].end;inv+=count[e]*(1+min(4,usage[id]));U x=cards[id].m;while(x){int z=ctz(x);x&=x-1;count[z]++;}}
  return inv;
 }
 void record(const State&start,const State&goal){
  State s=start;s.op.clear();script.clear();prefix.clear();prefix.reserve(goal.op.size());
  for(auto m:goal.op){
   prefix.push_back(s.a);
   int id=m.k,f=locate(s,id),d=m.d;
   int before=d==K-1?-1:s.a[d+(d>=f)],after=d==0?-1:s.a[d-1+(d-1>=f)];
   script.push_back({id,d,before,after});moveCard(s,f,d);
  }
 }
 bool replay(const State&start,const State&goal,int cut,int length,int mode,U tar,State&out){
  // All three destination conventions replay the untouched prefix identically.
  State s=start;s.a=prefix[cut];s.op.assign(goal.op.begin(),goal.op.begin()+cut);
  for(int t=cut+length;t<(int)script.size();t++){
   auto x=script[t];int f=locate(s,x.id),d=x.d;
   if(mode==1){int pos=x.bef<0?K:locate(s,x.bef);d=pos-(pos>f);}
   if(mode==2){int pos=x.aft<0?-1:locate(s,x.aft);d=pos+(pos<f);}
   if(d==f)continue;
   U m=cards[x.id].m;for(int j=0;j<f&&m;j++)m&=~cards[s.a[j]].m;if(!m)return false;
   moveCard(s,f,d);
  }
  if(display(s)!=tar)return false;out=std::move(s);return true;
 }
 void run(const State&start,State&best,U target){
  int reps=1,penalty=0,maxcut=2;
  for(int r=0;r<reps&&best.op.size()>1;r++){
   record(start,best);State winner=best;int oldE=energy(best),old=best.op.size(),bestMargin=0;
   for(int length=1;length<=maxcut;length++)for(int i=0;i+length<=old;i++){
    State cand;
    for(int mode=0;mode<3;mode++)if(replay(start,best,i,length,mode,target,cand)){
     int margin=(old-int(cand.op.size()))*K*3-penalty*(energy(cand)-oldE);
     if(margin>bestMargin){bestMargin=margin;winner=std::move(cand);}
     break;
    }
   }
   if(!bestMargin)break;best=std::move(winner);
  }
 }
};
bool guidedPrep(State&s,const vector<int>&seq,const State&guide,bool invert,int weighted){
 int rank[K];bool moved[K]={},active[K]={};
 U seen=0;for(int j=0;j<K;j++){int id=guide.a[j];rank[id]=j;if(cards[id].m&~seen)active[id]=true;seen|=cards[id].m;}
 for(auto m:guide.op)moved[m.k]=true;
 s.front=0;s.cover=0;bool chosen[8]={};
 for(int t=0;t<(int)seq.size();t++){
  int q=invert?int(seq.size())-1-t:t,id=seq[q],pos=locate(s,id);U options=cards[id].m&~s.cover;if(!options)return false;
  int score[64]={},best=INT_MAX,zbest=-1;
  for(int i=s.front;i<pos;i++){
   int b=s.a[i];U m=cards[b].m&options;int w=weighted?(moved[b]?4:13):8;
   while(m){int z=ctz(m);m&=m-1;score[z]+=w;}
  }
  while(options){int z=ctz(options);options&=options-1;if(score[z]<best)best=score[z],zbest=z;}
  U bit=1ULL<<zbest;
  for(int j=s.front;j<pos;){
   int b=s.a[j];if(!(cards[b].m&bit)){++j;continue;}
   int dest=pos;
   if(active[b]||moved[b]){
    int low=0,cost=0;for(int z=pos+1;z<K;z++){
     int a=s.a[z];cost+=(rank[a]>rank[b])-(rank[a]<rank[b]);if(cost<low)low=cost,dest=z;
    }
   }else dest=restoreAt(s,j,pos);
   moveCard(s,j,dest);pos--;
  }
  int d=count(chosen,chosen+q,true);moveCard(s,pos,d);chosen[q]=true;s.front++;s.cover|=cards[id].m;
 }
 return true;
}
struct Hint{array<uint8_t,3> ids{};int n=0;U cov=0;int est=0,rank=0;};
array<vector<Hint>,P> hints;
bool gripRoute(State&s,const Hint& h,int variant){
 array<int,3>q{0,1,2};int n=h.n;
 if(variant==1)reverse(q.begin(),q.begin()+n);
 if(variant==2)sort(q.begin(),q.begin()+n,[&](int i,int j){return cards[h.ids[i]].end<cards[h.ids[j]].end;});
 s.front=0;s.cover=0;bool done[3]={};
 for(int z=0;z<n;z++){
  int i=q[z],id=h.ids[i],d=0;for(int j=0;j<i;j++)d+=done[j];
  if(!fetch(s,id,d,~0ULL,false))return false;
  done[i]=true;s.front=z+1;s.cover|=cards[id].m;
 }
 return true;
}
void findHints(){
 int keep=8;if(!keep)return;int W=6;
 for(int a=0;a<P;a++){
  vector<Hint>b(1),all;
  for(int lev=0;lev<3;lev++){
   vector<Hint>next;
   for(auto&h:b){
    for(int i=0;i<K;i++){U m=cards[i].m&~h.cov;
     if(!m||((cards[i].v^A[a])&m))continue;
     Hint x=h;x.ids[x.n++]=i;x.cov|=m;
     int slot=home[i]%256; x.rank+=3+min(10,slot)*2;
     int residual=staticCost(~x.cov,a,false);
     int inner=pc(low&~x.cov);x.est=x.rank+residual+min(4,inner);
     next.push_back(x);
    }
   }
   sort(next.begin(),next.end(),[](auto&a,auto&b){return a.est<b.est;});
   b.clear();
   for(auto h:next){
    bool dup=false;for(auto&s:b)if(s.cov==h.cov){dup=true;break;}if(dup)continue;
    b.push_back(h);all.push_back(h);if(int(b.size())>=W)break;
   }
  }
  sort(all.begin(),all.end(),[](auto&a,auto&b){return a.est<b.est;});
  for(auto h:all){
   if(h.n<=2&&int(core[a].size())==h.n&&equal(core[a].begin(),core[a].end(),h.ids.begin()))continue;
   hints[a].push_back(h);if(int(hints[a].size())==keep)break;
  }
   // Retaining the two intended head sheets leaves the family's erased
   // coordinate for a third sheet. Search that third grip as part of the
   // core, so it can also repair the unprinted tail in the same pickup.
   if(core[a].size()==2){
    int x=core[a][0],b=core[a][1];U need=~(cards[x].m|cards[b].m),head=need&low;
    if(head){
     vector<Hint> remedies;
     for(int k=0;k<K;++k){U ink=cards[k].m&need;
      if((ink&head)!=head||(ink&(cards[k].v^A[a])))continue;
      Hint h;h.ids={uint8_t(x),uint8_t(b),uint8_t(k)};h.n=3;h.cov=~need|ink;
      h.rank=home[k]%256;h.est=staticCost(~h.cov,a,false)+2*h.rank;
      remedies.push_back(h);
     }
     stable_sort(remedies.begin(),remedies.end(),[](const Hint& l,const Hint& r){return l.est<r.est;});
     if(remedies.size()>3)remedies.resize(3);
     hints[a].insert(hints[a].begin(),remedies.begin(),remedies.end());
    }
   }

 }
}
array<vector<int>,P> rescueLists;
void makeRescues(){
 for(int a=0;a<P;a++){
  auto &seq=rescueLists[a];seq.clear();U cov=0;
  for(int id:core[a])if(cards[id].m&~cov){seq.push_back(id);cov|=cards[id].m;}
  while(cov!=~0ULL){
   int best=-1,score=INT_MIN;
   for(int id=0;id<K;id++){
    U ink=cards[id].m&~cov;if(!ink||((cards[id].v^A[a])&ink))continue;
    int gain=pc(ink)*100-(home[id]%256);if(gain>score)score=gain,best=id;
   }
   if(best<0)throw logic_error("uncertified design");
   seq.push_back(best);cov|=cards[best].m;
  }
 }
}
// 未移動紙を一時退避領域より前へ残し、退避済み対象を後続対象より前へ置く。
// 遮蔽紙の一時退避は1回、確定配置への移動も高々1回となる。
void rescue(State&s,int a){
 LOCAL_ONLY(trace_stats.count("rescue_calls");trace_stats.mark_fallback());
 auto&seq=rescueLists[a];array<int,K>rank;rank.fill(K);for(int i=0;i<(int)seq.size();i++)rank[seq[i]]=i;
 int boundary=K,front=0;U cov=0;s.op.clear();
 for(int id:seq){
  int p=locate(s,id);U exposed=cards[id].m&~cov;int z=ctz(exposed);U bit=1ULL<<z;
  for(int j=front;j<min(p,boundary);){
   int k=s.a[j];if(!(cards[k].m&bit)){j++;continue;}
   int d=K-1;
   if(rank[k]<K){int pos=boundary;while(pos<K&&rank[s.a[pos]]<rank[k])pos++;d=pos-1;}
   moveCard(s,j,d);boundary--;p=locate(s,id);
  }
  bool staged=p>=boundary;moveCard(s,p,front);front++;boundary+=staged;cov|=cards[id].m;
 }
 if(display(s)!=A[a]||s.op.size()>K+64)abort();
}
struct DesignSnap {
 int groupCount;
 array<int,K> uses;
 array<U,P>codes;array<Card,K>sh;array<int,K>rank;array<vector<int>,P>anchors;array<int,N>map;int h;
 void save(){groupCount=DonorGroups;copy_n(usage,K,uses.begin());copy_n(A,P,codes.begin());copy_n(cards,K,sh.begin());copy_n(home,K,rank.begin());anchors=core;map=bitMap;h=H;}
 void load()const{setGroupCount(groupCount);copy_n(uses.begin(),K,usage);copy_n(codes.begin(),P,A);copy_n(sh.begin(),K,cards);copy_n(rank.begin(),K,home);core=anchors;bitMap=map;H=h;low=lo(H);}
};
// 人工注文後の積層順から、各取得窓を遮る紙枚数を推定する。
void empiricalSlots(const vector<pair<int,int>>& slot,const array<int,K>& match,const array<U,K>& payload,
   const array<U,K>& actual,const array<int,K>& depth,const array<vector<int>,K>& served){
 const int coef=layout_prices[3];
 int ns=6,cap=40,benefit=100,clamp=16;
 State st;for(int p=0;p<K;p++)st.a[p]=p;
 stable_sort(st.a.begin(),st.a.end(),[](int i,int j){return home[i]<home[j];});
 int owner[K];for(int id=0;id<K;id++)owner[match[id]]=id;
 static int accum[K][K];memset(accum,0,sizeof(accum));
 unsigned save=rng.x;rng.x=746371;
 for(int rep=0;rep<ns;rep++){
   for(int q=0;q<6;q++){
     int a=rng(P);st.op.clear();st.front=0;st.cover=0;State next=st;int best=10000;
     auto test=[&](State s){if(taildp.solve(s,A[a])&&int(s.op.size())<best){best=s.op.size();next=std::move(s);}};
     test(st);State x=st;if(fetchCore(x,core[a],false))test(x);
     if(core[a].size()==2){State y=st;if(fetchCore(y,core[a],true))test(y);}
     st=std::move(next);
   }
   int pos[K];alignas(64) static U bucket[K][48];int counts[64]={};
   for(int p=0;p<K;p++){
     int id=st.a[p];pos[id]=p;
     fill(bucket[p],bucket[p]+cap+1,0);
     for(int z=0;z<64;z++)bucket[p][min(cap,counts[z])]|=1ULL<<z;
     for(int t=1;t<=cap;t++)bucket[p][t]|=bucket[p][t-1];
     U m=cards[id].m;while(m){int z=ctz(m);m&=m-1;counts[z]++;}
   }
   auto grip=[&](int p,U m){
     if(!m)return cap;
#if defined(__AVX512F__)
     // Most printable endpoint choices expose a grip in the first eight
     // strata. Read their cumulative masks together and take the first hit.
     unsigned hits=_mm512_test_epi64_mask(_mm512_load_si512(bucket[p]),_mm512_set1_epi64(m));
     if(hits)return int(countr_zero(hits));
     int lo=7,hi=cap;
#else
     if(bucket[p][0]&m)return 0;
     int lo=0,hi=cap;
#endif
     while(hi-lo>1){int mid=(hi+lo)/2;if(bucket[p][mid]&m)hi=mid;else lo=mid;}return hi;
   };
   // 同じ端点では紙の模様が同じなので、相方の取得費を棚の位置ごとに計算し直す必要がない。
   int baseGrip[K];for(int k=0;k<K;k++)baseGrip[k]=grip(pos[k],cards[k].m);
   for(int id=0;id<K;id++){
    const auto& users=served[id];int last=-1;U m=0;
    struct Partner{U cut;int base,after;};
    array<Partner,P> paired;int pairs=0,singles=0;
    for(int t=0;t<K;t++)if(costMatrix[id][t]<500000){
     int z=slot[t].first,p=pos[owner[t]];
     if(z!=last){
      last=z;m=payload[id]|(actual[id]&~low&lo(z+1));pairs=singles=0;
      for(int a:users){
       if(core[a].size()==1){++singles;continue;}
       int j=core[a][0]==id?core[a][1]:core[a][0];
       U other=cards[j].m;paired[pairs++]={m&~other,baseGrip[j],grip(pos[j],other&~m)};
      }
     }
     int first=grip(p,m),sum=singles*first;
     for(int i=0;i<pairs;i++){auto v=paired[i];int both=min(first+v.after,v.base+grip(p,v.cut));
      sum+=min(both,clamp+min(first,v.base));
     }
     accum[id][t]+=sum;
    }
   }
 }
 rng.x=save;
 for(int id=0;id<K;id++)for(int t=0;t<K;t++)if(costMatrix[id][t]<500000){
   int old=int(served[id].size())*depth[t];
   costMatrix[id][t]+=coef*(accum[id][t]*100/ns-old*benefit)/100;
 }
}

struct ShelfExample{
 array<uint8_t,K> stack;
 array<U,3> moved{};
 int product;
};
static void crossEndpointShelf(array<uint8_t,K>& order,const vector<ShelfExample>& examples){
 using Set=array<U,3>;
 struct Request{Set options{};int weight=0,least=K;};
 vector<Request> reqs;array<vector<int>,K> uses;
 for(const auto& ex:examples){
  U seen=0,target=A[ex.product];
  for(int k:ex.stack){
   U rem=~seen,ink=rem&cards[k].m;if(!ink)continue;
   bool primary=k==core[ex.product][0]||(core[ex.product].size()==2&&k==core[ex.product][1]);
   Request q;q.weight=1+bool(ex.moved[k/64]>>(k%64)&1);
   for(int v=0;v<K;v++)if(k==v||(!primary&&(ink&~cards[v].m)==0&&((cards[v].v^target)&cards[v].m&rem)==0)){
    q.options[v/64]|=U(1)<<(v%64);uses[v].push_back(reqs.size());
   }
   reqs.push_back(q);seen|=ink;if(seen==~U(0))break;
  }
 }
 array<Set,K+1> level{};array<int,K> depth,origin;
 array<array<uint8_t,N>,K+1> above{};
 for(int i=0;i<K;++i){
  int k=order[i];origin[k]=i;above[i+1]=above[i];
  int d=K;for(U b=cards[k].m;b;b&=b-1){int z=ctz(b);d=min(d,int(above[i+1][z]++));}
  depth[k]=d;level[d][k/64]|=U(1)<<(k%64);
  for(int r:uses[k])reqs[r].least=min(reqs[r].least,d);
 }
 auto intersects=[](const Set& a,const Set& b){return (a[0]&b[0])|(a[1]&b[1])|(a[2]&b[2]);};
 vector<int> affected,visit(reqs.size());int stamp=0;
 // Only the adjacent pair has a different set of sheets above it. Its grip
 // depths change by at most one, which also bounds each request's new minimum.
 for(int sweep=0;sweep<6;++sweep){
  int changed=0;
  for(int j=0;j<K-1;++j){
   int i=sweep%2?K-2-j:j,k=order[i],v=order[i+1];
   int dk=K,dv=K;
   for(U b=cards[v].m;b;b&=b-1)dv=min(dv,int(above[i][ctz(b)]));
   for(U b=cards[k].m;b;b&=b-1){int z=ctz(b);dk=min(dk,int(above[i][z])+int(cards[v].m>>z&1));}
   if((dk==depth[k]&&dv==depth[v])||abs(i+1-origin[k])>6||abs(i-origin[v])>6)continue;
   auto toggle=[&](int a,int d){level[d][a/64]^=U(1)<<(a%64);};
   auto changeLevels=[&](){toggle(k,depth[k]);toggle(k,dk);toggle(v,depth[v]);toggle(v,dv);};
   changeLevels();affected.clear();++stamp;
   for(int a:{k,v})for(int r:uses[a])if(visit[r]!=stamp){visit[r]=stamp;affected.push_back(r);}
   auto minimum=[&](const Request& q){
    if(q.least&&intersects(q.options,level[q.least-1]))return q.least-1;
    return q.least+!intersects(q.options,level[q.least]);
   };
   int delta=abs(i+1-origin[k])+abs(i-origin[v])-abs(i-origin[k])-abs(i+1-origin[v]);
   for(int r:affected){auto& q=reqs[r];delta+=2*q.weight*(minimum(q)-q.least);}
   if(delta>=0){changeLevels();continue;}
   for(int r:affected)reqs[r].least=minimum(reqs[r]);
   swap(order[i],order[i+1]);depth[k]=dk;depth[v]=dv;
   above[i+1]=above[i];for(U b=cards[v].m;b;b&=b-1)++above[i+1][ctz(b)];
   ++changed;
  }
  if(!changed)break;
 }
}

struct ShelfRequests {
 vector<ShelfExample> synthetic;
 array<vector<int>,N> group;array<vector<uint32_t>,N> requested;
 array<int,K> local{};array<uint8_t,K> order,anchor;int examples=0;
 explicit ShelfRequests(const State& initial):order(initial.a),anchor(initial.a){
  for(int k:order){auto &g=group[cards[k].end];local[k]=g.size();g.push_back(k);}
 }
 void observe(const State& final,int a){
  ++examples;
  bool moved[K]{};for(auto mv:final.op)moved[mv.k]=true;U seen=0,target=A[a];
  if(examples<=P){
   ShelfExample ex;ex.stack=final.a;ex.product=a;
   for(auto mv:final.op)ex.moved[mv.k/64]|=U(1)<<(mv.k%64);
   synthetic.push_back(ex);
  }
  for(int k:final.a){
   U rem=~seen,ink=cards[k].m&rem;if(!ink)continue;int z=cards[k].end;
   if(group[z].size()<=16){
    uint32_t options=1u<<local[k];
    if(k!=core[a][0]&&(core[a].size()<2||k!=core[a][1])){
     for(int q:group[z]){
      auto c=cards[q];if(!(ink&~c.m)&&!(rem&c.m&(c.v^target)))options|=1u<<local[q];
     }
    }
    requested[z].push_back(options);
    if(moved[k])requested[z].push_back(options);
   }
   seen|=ink;if(!~seen)break;
  }
 }
 void arrange(){
  order=anchor;const int inertia=max(1,(examples+128)/256);
  array<int,K> place{};for(int i=0;i<K;i++)place[order[i]]=i;
  for(int z=0;z<N;z++){
   auto &g=group[z];int n=g.size();if(n<2||n>16||requested[z].empty())continue;
   int full=(1<<n)-1;vector<int> weight(full+1),dp(full+1,INT_MAX);
   vector<uint8_t> last(full+1);
   for(unsigned m:requested[z])weight[m]++;
   for(int j=0;j<n;j++)for(int m=0;m<=full;m++)if(m>>j&1)weight[m]+=weight[m^(1<<j)];
   dp[0]=0;
   for(int m=0;m<full;m++){
    int t=popcount(unsigned(m));int dist=t+1<n?min(3,place[g[t+1]]-place[g[t]]):0;
    for(unsigned left=full^m;left;left&=left-1){
     int j=countr_zero(left),u=m|(1<<j);
     // Regularize changes to sparse synthetic observations toward the shelf
     // that already reflects depth and the complementary partner.
     int v=dp[m]+dist*weight[full^u]+inertia*(1+usage[g[j]])*abs(t-j);
     if(v<dp[u]||(v==dp[u]&&j==t)){dp[u]=v;last[u]=j;}
    }
   }
   for(int m=full;m;){int j=last[m],t=popcount(unsigned(m))-1;order[place[g[t]]]=g[j];m^=1<<j;}
  }
 }
};
shared_ptr<ShelfRequests> runningRequests;array<uint8_t,K> learnedShelf;bool haveLearnedShelf=false;

using DayShelves=array<array<uint8_t,K>,4>;using DayProbe=array<array<float,8>,P>;DayShelves sampledShelves;DayProbe sampledProbe;
vector<double> layoutFeatures(const vector<int>& qs,array<uint8_t,K>& suggested,shared_ptr<ShelfRequests>& observations){
 array<array<double,12>,3> acc{};array<double,10> staticSum{};
 State s;for(int i=0;i<K;i++)s.a[i]=i;
 stable_sort(s.a.begin(),s.a.end(),[](int i,int j){return home[i]<home[j];});
 // The design-specific shelf and synthetic demand history must be discarded
 // before the next paper table is constructed.
 arrangeShelf(s);observations=make_shared<ShelfRequests>(s);ShelfRequests& requests=*observations;pricing=GripPrice{};pricing.init();frozenGeometry=true;
 taildp.resetGeometry();
 for(int a=0;a<P;a++){
  int p=core[a][0];U m=cards[p].m;int role=core[a].size()==1?0:core[a][1]>=K-DonorCount?1:2;
  if(core[a].size()>1)m|=cards[core[a][1]].m;
  staticSum[role*3]+=pc(~m);
  staticSum[role*3+1]+=staticCost(~m,a);
  staticSum[role*3+2]+=home[p]%256;
  staticSum[9]+=pc((~m)&low);
 }
 int turn=0;
 for(int a:qs){
  pricing.update(s,turn);++queryStamp;
  s.op.clear();s.front=0;s.cover=0;State out=s;int best=999,prep=0;
  auto test=[&](State u){int cost=u.op.size();if(taildp.solve(u,A[a])&&int(u.op.size())<best){prep=cost;best=u.op.size();out=std::move(u);}};
  test(s);State x=s;if(fetchCore(x,core[a],false))test(x);
  if(core[a].size()==2){State y=s;if(fetchCore(y,core[a],true))test(y);}
  if(core[a].size()==2){
   State partial=s;if(fetch(partial,core[a][0],0,~U(0),true))test(partial);
  }
  if(best==999)best=40;else {Sched sc;sc.run(s,out,2);best=out.op.size();}
  int role=core[a].size()==1?0:core[a][1]>=K-DonorCount?1:2;
  auto& z=acc[role];++z[0];z[1]+=best;z[2]+=prep;z[3]+=max(0,best-12);z[4]+=best*best;
  bool moved[K]{};for(auto m:out.op)moved[m.k]=true;
  U cover=0;int visible=0,active=0,extent=0;
  for(int id:out.a){
   if(!~cover)break;
   ++extent;if(cards[id].m&~cover){++visible;active+=moved[id];}cover|=cards[id].m;
  }
  z[5]+=visible;z[6]+=active;z[7]+=extent;
  int first=core[a][0];z[8]+=pc(cards[first].m);z[9]+=home[first]%256;z[10]+=int(moved[first]);z[11]+=int(out.op.size())-int(count(moved,moved+K,true));
  sampledProbe[a]={float(best),float(prep),float(visible),float(active),float(extent),float(moved[first]),float(role),float(pc(cards[first].m))};
  requests.observe(out,a);pricing.observe(out,turn++);s=std::move(out);
  if(!(turn&63)&&turn<=256)sampledShelves[turn/64-1]=s.a;
 }
 vector<double> feature;
 for(double x:staticSum)feature.push_back(x/P);
 for(auto& z:acc)for(double x:z)feature.push_back(x/qs.size());
 frozenGeometry=false;shelfReady=false;pricing=GripPrice{};queryStamp=0;
 fill_n(cacheStamp,K,0);
 taildp.resetGeometry();
 requests.arrange();suggested=requests.order;return feature;
}

// 注文前に公開商品IDから人工列を作る。追加土台で他商品の共通紙を共有するときは、
// 全担当辺の表示順を満たす最大支持を計算し直す。
// Order windows from the joint pair graph before choosing printed sheets.
array<int,N> jointWindowDifficulty(){
 array<int,N> difficulty{};
 H=36;low=lo(H);
 array<bool,P> paired{};int used=0;
 for(const auto& e:joint6064::coarse_pairs(paired)){
  if(paired[e.a]||paired[e.b])continue;
  paired[e.a]=paired[e.b]=true;
  for(U m=e.differences;m;m&=m-1)++difficulty[ctz(m)];
  if(++used==64)break;
 }
 return difficulty;
}
array<int,N> orderJointWindows(const array<int,N>& difficulty,int tie){
 array<int,N> order{};iota(order.begin(),order.end(),0);
 if(tie){mt19937 bits(tie);shuffle(order.begin(),order.end(),bits);}
 stable_sort(order.begin(),order.end(),[&](int a,int b){return difficulty[a]<difficulty[b];});
 return order;
}

static double rankLayout(const vector<double>& f){
constexpr array<double,46> model={81.8260738,20.870748,9.29697489,83.3885928,14.1880983,58.2445365,70.6872701,-0.176354087,-144.611908,-132.528461,0,64.8726179,142.677776,39.6648672,-0.365352757,108.718007,-186.387246,0.298360711,-81.7386206,9.29697489,-150.729759,685.260301,82.8296613,35.0855582,35.1060892,23.5500553,0.607646915,57.9727928,-21.2948639,-1.35379174,-27.8361367,58.2445365,820.470233,81.6125594,-82.8296613,61.0135543,392.996561,120.244772,0.592314481,30.5729887,188.435138,-5.07533372,-27.0510707,-144.611908,-130.102742,509.448451};
 return inner_product(f.begin(),f.end(),model.begin(),0.);
}
// Public product features; all positions refer to the actual printed initial shelf.
namespace optical_price {
using Word=unsigned long long;
constexpr int sheets=192,features=24;
using Row=std::array<double,features>;
struct Paper{
 std::array<Word,sheets> ink{},color{},gripWindows{};
 std::array<int,sheets> depth{};
 void prepare(){
  std::array<int,64> above{};
  for(int i=0;i<sheets;++i){
   int d=sheets;Word windows=0;
   for(Word v=ink[i];v;v&=v-1){
    int z=std::countr_zero(v),p=above[z]++;
    if(p<d){d=p;windows=0;}
    if(p==d)windows|=Word(1)<<z;
   }
   depth[i]=d;gripWindows[i]=windows;
  }
 }
 Row product(Word target)const{
  Row x{};for(int j=2;j<16;j++)x[j]=1000;
  int nfirst=0,npair=0,near8=0,near16=0;
  auto option=[&](int moves,int rem,int grips,int ranks,int first){
   double depthCost=grips+moves,rankCost=ranks/32.;
   x[4]=std::min(x[4],double(rem));
   x[5]=std::min(x[5],depthCost+rankCost*.25);
   constexpr double residual[]={.15,.30,.60,1.2,.15,.30,.60,1.2};
   for(int j=0;j<8;j++){
    double score=rem*residual[j]+depthCost*(j<4?1:.35)+rankCost*.4;
    x[6+j]=std::min(x[6+j],score);
   }
   for(int j=0;j<2;j++){
    double score=depthCost+(j?.8:.27)*rem+.22*rankCost;
    if(score<x[14+j]){
     x[14+j]=score;x[16+3*j]=rem;x[17+3*j]=grips;x[18+3*j]=first;
    }
   }
   near8+=rem<=8;near16+=rem<=16;
  };
  for(int i=0;i<sheets;i++){
   if(!ink[i]||((color[i]^target)&ink[i]))continue;
   ++nfirst;Word rest=~ink[i];
   x[2]=std::min(x[2],double(depth[i]));x[3]=std::min(x[3],double(std::popcount(rest)));
   option(1,std::popcount(rest),depth[i],i,i);
   for(int j=0;j<sheets;j++){
    Word fresh=ink[j]&rest;
    if(!fresh||((color[j]^target)&fresh))continue;
    ++npair;
    // Removing the shallower anchor opens one of the other's cheapest grips
    // exactly when that anchor covers a minimum-depth window.
    bool shared=i<j?(ink[i]&gripWindows[j])!=0:(ink[j]&gripWindows[i])!=0;
    option(2,std::popcount(rest&~fresh),depth[i]+depth[j]-shared,i+j,i);
   }
  }
  x[0]=std::log1p(nfirst);x[1]=std::log1p(npair);
  x[22]=std::log1p(near8);x[23]=std::log1p(near16);
  return x;
 }
};
}

// 人工注文後の棚で2枚の取得に共通する遮蔽紙を数える。これは予測用の近似量である。
namespace shelf_cost {
constexpr int extra = 9, width = optical_price::features+2*extra;
using Features = array<array<float,width>,P>;
struct Route { int first,second,rem; };
struct Grip { array<U,3> blockers{}; U windows=0; int depth=0; };
struct Choices {
 array<Grip,4> opt{};int n=0;
 void add(const array<U,3>& blockers,U bit,int depth){
  for(int i=0;i<n;++i)if(opt[i].blockers==blockers){opt[i].windows|=bit;return;}
  if(n==int(opt.size())&&depth>=opt.back().depth)return;
  int at=min(n,int(opt.size())-1);
  while(at&&opt[at-1].depth>depth){opt[at]=opt[at-1];--at;}
  opt[at]={blockers,bit,depth};n=min(n+1,int(opt.size()));
 }
};
Features measure(const Card* sh,const U* targets,const array<uint8_t,K>& initial,const DayShelves& samples){
 Features result{};
 optical_price::Paper paper;
 for(int i=0;i<K;++i){paper.ink[i]=sh[initial[i]].m;paper.color[i]=sh[initial[i]].v;}
 paper.prepare();
 array<vector<Route>,P> menu;
 for(int a=0;a<P;++a){
  auto x=paper.product(targets[a]);copy(x.begin(),x.end(),result[a].begin());
  for(int k=0;k<K;++k){
   U ink=sh[k].m;if(!ink||((sh[k].v^targets[a])&ink))continue;
   U rest=~ink;menu[a].push_back({k,-1,pc(rest)});
   for(int j=0;j<K;++j){
    U more=sh[j].m&rest;if(more&&!((sh[j].v^targets[a])&more))
      menu[a].push_back({k,j,pc(rest&~more)});
   }
  }
 }
 array<array<float,extra>,P> lowValue{},highValue{};
 for(auto& row:lowValue)row.fill(1e6f);
 for(const auto& order:samples){
  array<Choices,K> choices{};array<array<U,3>,N> above{};array<int,N> depth{};
  array<int,K> pos{};
  for(int i=0;i<K;++i){
   int k=order[i];pos[k]=i;
   for(U m=sh[k].m;m;m&=m-1){
    int z=ctz(m);choices[k].add(above[z],U(1)<<z,depth[z]);
    above[z][k/64]|=U(1)<<(k%64);++depth[z];
   }
  }
  for(int a=0;a<P;++a){
   array<float,extra> f{};f[0]=f[1]=f[2]=f[3]=1000;int pairs=0;
   for(auto route:menu[a]){
    int k=route.first,j=route.second,work=K+2,saved=0;
    const auto& ck=choices[k];
    if(j<0){work=ck.opt[0].depth+1;f[0]=min(f[0],float(work));}
    else{
     const auto& cj=choices[j];
     for(int u=0;u<ck.n;++u)for(int v=0;v<cj.n;++v){
      const Grip &g=ck.opt[u], &h=cj.opt[v];
      if(!(g.windows&~sh[j].m)&&!(h.windows&~sh[k].m))continue;
      // 遮蔽紙と担当2枚の和集合。相方紙が遮蔽集合に含まれる場合も二重に数えない。
      auto used=g.blockers;for(int b=0;b<3;++b)used[b]|=h.blockers[b];
      used[k/64]|=U(1)<<(k%64);used[j/64]|=U(1)<<(j%64);
      int paid=pc(used[0])+pc(used[1])+pc(used[2]);
      if(paid<work){work=paid;saved=g.depth+h.depth+2-paid;}
     }
     if(work==K+2)continue;
     ++pairs;
    }
    float rank=float(pos[k]+(j<0?0:pos[j]))/K;
    float base=work+.25f*rank;
    f[1]=min(f[1],base+.18f*route.rem);f[3]=min(f[3],base+.8f*route.rem);
    float mid=base+.4f*route.rem;
    if(mid<f[2]){f[2]=mid;f[4]=route.rem;f[5]=work;f[6]=saved;f[7]=rank;}
   }
   f[8]=log1pf(float(pairs));
   for(int i=0;i<extra;++i){
    result[a][24+i]+=f[i]/samples.size();
    lowValue[a][i]=min(lowValue[a][i],f[i]);highValue[a][i]=max(highValue[a][i],f[i]);
   }
  }
 }
 for(int a=0;a<P;++a)for(int i=0;i<extra;++i)result[a][24+extra+i]=highValue[a][i]-lowValue[a][i];
 return result;
}
}

namespace geometry_price{
constexpr int width=52;
using Features=array<array<float,width>,P>;
Features features(const Card* cards,const U* targets,const array<uint8_t,K>& shelf,
                  const DayShelves& samples,const DayProbe& probe,double legacy,int g){
 auto optics=shelf_cost::measure(cards,targets,shelf,samples);
 Features x{};for(int a=0;a<P;++a){copy(optics[a].begin(),optics[a].end(),x[a].begin());
  copy(probe[a].begin(),probe[a].end(),x[a].begin()+42);x[a][50]=legacy;x[a][51]=g;}
 return x;
}
}
namespace geometry_model {
double score(const geometry_price::Features& x,array<float,P>& items){
 static constexpr float mu[]={0.693147242f,1.29751694f,3.01836467f,19.9479465f,11.4583406f,4.77111101f,8.04315662f,10.1569433f,13.997571f,21.3634739f,5.0118084f,6.94567776f,10.6820526f,17.7912159f,9.15668964f,15.8016529f,13.0961723f,3.29034448f,96.3515625f,12.1716032f,3.40821457f,96.3515625f,0.664107025f,1.10714078f,4.71810961f,8.19046497f,11.252408f,16.1728935f,12.7922106f,5.98129559f,0.370688975f,0.616912067f,1.29609621f,1.47804654f,1.58791912f,1.85209775f,1.95823169f,1.36790478f,1.95132649f,0.848107696f,0.131631851f,0.000333350414f,11.8650141f,3.29760408f,12.0920715f,2.66099453f,170.521103f,0.996656597f,0.687786579f,44.0520515f,795.285156f,6.45147038f};
 static constexpr float inv[]={5570.26886f,1.21360285f,0.37005539f,0.0898112439f,0.129483001f,0.342276767f,0.509218351f,0.595601742f,0.37153563f,0.153286175f,1.55136494f,0.842515633f,0.320417585f,0.134398092f,0.589535754f,0.241951876f,0.142118931f,0.386903429f,0.0190726084f,0.140113245f,0.382097697f,0.0190726084f,0.984777209f,0.981333094f,0.391491346f,0.58680758f,0.426252399f,0.207972453f,0.139883973f,0.48598593f,2.47818173f,3.17313666f,1.21574408f,0.902693702f,0.874266859f,0.792944928f,0.767928405f,0.373816856f,0.780920919f,1.2332891f,4.27268725f,102.115335f,0.300352412f,0.348444806f,0.16377426f,0.791418804f,0.0290640679f,17.3233319f,1.60395506f,0.0898112439f,0.0201175169f,2.0094876f};
 static constexpr float weight[]={-0.0337081887f,1.62152803f,1.07226062f,-0.0259707998f,-0.142379597f,0.976762295f,-0.41196543f,-0.182246312f,0.626238585f,-0.42518723f,-0.307709664f,-0.0755233914f,0.0368120298f,-0.336631924f,1.05578494f,0.0744922906f,-0.161761954f,-0.871343732f,-0.117099874f,-0.266686022f,-0.192442849f,-0.117099874f,0.0396717675f,0.017896045f,-0.223979771f,0.277509391f,-0.607356369f,2.00934458f,-0.257494479f,-0.359936655f,-0.00554431183f,-0.193364665f,-1.69922578f,-0.0106935501f,-0.00659013027f,0.093134068f,-0.0750804096f,-0.045144137f,-0.0264679603f,0.0073156273f,0.0220358744f,0.0075232801f,0.196966127f,0.265894592f,0.322709769f,-0.0137691554f,0.0628089383f,-0.00678976718f,0.0633700415f,0.0259707812f,0.034613125f,-0.0115543194f};
 static constexpr float hidden[32][52]={{0.146595702f,0.143753484f,-0.145081758f,-0.342641026f,-0.303791672f,0.176732242f,-0.0227309726f,-0.127851933f,0.061208427f,-0.0995093808f,-0.275649071f,0.0185228456f,-0.0982599705f,-0.124483652f,0.0425527878f,-0.265225947f,0.00248706667f,-0.0690271929f,0.316456378f,-0.367763877f,0.122416899f,0.446899921f,-0.0477800444f,0.211560979f,-0.209698379f,-0.223555654f,-0.0792057663f,-0.00770292897f,0.257928431f,-0.35984835f,0.0558079332f,-0.0304407179f,0.124579363f,0.0117530748f,-0.0116505437f,-0.0251408108f,-0.11054194f,0.0434115343f,-0.0150890984f,-0.1059236f,0.044412829f,-0.0421559736f,0.227134615f,-0.225155056f,0.373708248f,-0.120549053f,0.0844915211f,-0.000640747137f,-0.226645112f,0.207212403f,0.023768628f,0.0363149568f},{-0.000456136331f,-0.21563907f,-0.489026338f,-0.082386896f,-0.102792524f,-0.491550118f,-0.190852001f,-0.0399921723f,-0.00387035427f,0.057810735f,0.261692524f,-0.147549734f,-0.128604457f,-0.163210899f,-0.177930713f,0.0598391332f,-0.163934574f,-0.134789303f,0.0859394446f,-0.0421728641f,0.0673799962f,-0.135069117f,-0.209337816f,-0.189409643f,0.109478012f,-0.155920923f,0.0285777152f,-0.246862352f,-0.241453543f,0.124554314f,0.0960834175f,-0.0796853155f,-0.0406639017f,-0.0580381043f,-0.0186023917f,0.0878666863f,0.0480861738f,0.00764143001f,-0.050487332f,0.0103526646f,-0.207485527f,-0.0376571268f,-0.0488640927f,-0.4281106f,0.0260630287f,0.205228835f,0.0542936362f,0.0778374374f,0.217583001f,-0.0236385874f,0.0142675936f,0.000593383913f},{-0.00160721643f,-0.00725984853f,0.702780545f,0.0550802127f,0.0999588445f,0.517440319f,0.390999287f,0.0907201543f,0.0648201704f,0.000201782284f,-0.00343781244f,-0.0634665415f,-0.0863411278f,0.0179971457f,0.123429842f,0.151077405f,-0.0578665696f,-0.0198411047f,-0.179010674f,-0.0171900969f,0.0262703281f,-0.150036111f,-0.181135535f,0.0627288222f,-0.020374557f,-0.070997484f,-0.0955923572f,-0.2348333f,-0.278162718f,0.0425585434f,-0.0371870585f,-0.135432854f,-0.000131706911f,-0.0170719698f,-0.114728995f,0.035587538f,0.0389403328f,-0.0470351428f,0.0632736832f,0.105846159f,-0.0983817354f,-0.552524567f,-0.122170717f,-0.411421359f,0.14869009f,0.10399165f,0.17525211f,-0.0255888831f,-0.115218163f,-0.0204893164f,-0.0326014906f,0.0359573476f},{-0.393904924f,-0.0142290508f,-0.0577940568f,0.181797728f,-0.0404412076f,-0.13729848f,-0.0203668159f,0.113438152f,0.0525582694f,0.00790594704f,0.142410234f,-0.0586371198f,-0.163096398f,0.0440624543f,0.185230762f,0.0811754614f,-0.0356336124f,0.175556391f,0.085003987f,0.123152092f,0.346763283f,0.018407939f,-0.118501253f,0.127999306f,-0.00990104768f,-0.0626235977f,0.0808207095f,-0.397491127f,-0.377117574f,0.191922173f,-0.129177004f,-0.0370194502f,0.202295601f,-0.0726729631f,-0.177454695f,0.0797233805f,0.0728055164f,0.112693653f,0.0832049325f,-0.0178596172f,-0.0811293796f,0.159701541f,-0.032478489f,-0.533461988f,-0.13580966f,0.0792418346f,0.0290871095f,0.0182509553f,0.0417336114f,-0.0787487775f,-0.0386486165f,-0.100188941f},{0.108670883f,0.182409391f,0.0789059848f,-0.254426003f,0.0281852726f,0.229652569f,-0.0709902644f,0.0150307771f,0.0562220141f,-0.202355519f,-0.0224818923f,-0.0675310344f,0.0356922597f,-0.131394044f,0.140364528f,-0.157817125f,-0.128626987f,0.0789019763f,0.159078181f,-0.271519661f,0.0316982195f,0.28387329f,0.1972031f,0.0797853842f,0.103277527f,-0.0977094248f,-0.220355585f,-0.0329038836f,-0.143837228f,-0.0374773592f,0.123083852f,0.114004694f,0.0914525837f,0.00334897405f,-0.0841201767f,0.0333868712f,0.0503860451f,-0.267073274f,0.00120891677f,-0.120962441f,0.127601564f,0.102017909f,0.215838775f,-0.150951281f,0.361405134f,0.119636148f,0.160728663f,0.0883664712f,0.0536841154f,0.258568376f,0.000888483832f,0.0706789792f},{0.0571664833f,0.0732751563f,0.0546527579f,-0.320972979f,-0.218924671f,0.150169164f,0.0663119629f,-0.120604761f,-0.125325143f,-0.235567689f,-0.164864644f,-0.112779938f,-0.093949005f,-0.191081882f,0.168970734f,-0.235720828f,-0.12651968f,0.132941335f,0.0411576889f,-0.355012983f,-0.00777647085f,0.116923265f,0.123221375f,-0.0037901511f,0.152127549f,-0.00651947269f,0.00101910939f,0.225720331f,0.168549255f,-0.0455042943f,-0.0190491099f,-0.163391262f,0.0785949603f,0.0386573076f,0.133370951f,0.088238433f,-0.086440444f,0.220550179f,0.141621068f,-0.0147606609f,-0.115211919f,-0.013968843f,0.378038198f,-0.263523966f,0.465789258f,-0.19675225f,0.00363145606f,0.00701744808f,-0.215335786f,0.44254759f,0.0733552501f,-0.0647172183f},{-0.052059032f,-0.181104973f,0.217171192f,-0.10536477f,0.405073524f,0.145657554f,0.401719838f,-0.208140269f,-0.00449110521f,0.175190151f,0.189623296f,-0.0381410532f,0.190078855f,0.335038722f,-0.0485110804f,0.0823520496f,-0.0920706168f,-0.254240781f,-0.0665918589f,0.170278549f,-0.343098402f,-0.00069113163f,0.13020876f,0.00612331787f,0.20933032f,0.0986514539f,-0.196012169f,-0.0670282245f,-0.135377243f,-0.133969888f,-0.0302031152f,0.197285697f,-0.141517401f,0.0592043735f,-0.0969606116f,-0.0172116328f,0.0397683159f,-0.0321089476f,0.0766125321f,-0.0634985566f,0.00360492943f,0.0160783604f,0.0564439222f,-0.48187235f,0.368830442f,-0.0355020277f,0.078991957f,0.025127966f,-0.017987268f,-0.130698457f,-0.0709516406f,-0.0223205928f},{0.0749567524f,0.192798227f,0.161262318f,0.0230658855f,-0.0705569908f,0.0095599629f,-0.0265982859f,0.0503110997f,0.121963248f,0.148458704f,-0.304194003f,-0.158670828f,0.0209258255f,-0.0603123978f,0.0559124351f,0.227083474f,0.0742644221f,0.199510261f,-0.316143215f,-0.0725517571f,0.0103603164f,-0.25756225f,0.203332365f,0.100420952f,-0.112514019f,0.00473452359f,0.0563705526f,0.196294367f,0.142773941f,0.0869284943f,0.0737246275f,-0.374590665f,0.0355645046f,-0.0582908653f,0.021074811f,0.0100329602f,-0.0290864557f,0.0298233368f,-0.0198256653f,0.0139382156f,0.158482969f,0.103340946f,-0.444041848f,0.486351818f,-0.565137923f,0.0393707305f,-0.146527812f,0.108573809f,-0.293675154f,-0.0718620121f,-0.126388028f,0.117708221f},{0.113947012f,-0.0150037622f,-0.36286971f,-0.0269161034f,-0.452858567f,-0.339799643f,-0.597899616f,0.153798193f,0.0729151294f,-0.169776157f,0.22291328f,0.107398838f,-0.247996449f,-0.325701237f,0.0277266521f,-0.039090842f,0.168497726f,0.142839611f,-0.120180182f,-0.221910045f,0.420592308f,0.0776981488f,0.237763748f,-0.0365766026f,-0.174454391f,-0.112129942f,0.212572619f,0.416744709f,0.390702575f,0.0293706711f,0.0362797603f,-0.136958808f,-0.121201955f,0.0344714448f,-0.0453203097f,-0.0134380385f,-0.0266430657f,-0.14009355f,0.0137032988f,0.00845338404f,0.00457041524f,0.0108440155f,0.0625297651f,0.406564325f,-0.373383373f,-0.248376518f,-0.0261023398f,0.0627311021f,0.115582116f,0.0801228359f,0.0158665273f,-0.0280485079f},{0.010192208f,0.106474809f,0.393230736f,0.130590975f,0.0760991648f,0.304919869f,0.273201674f,-0.0706423521f,-0.120809555f,0.040854279f,0.0523568504f,-0.0549954176f,0.189334154f,0.330662489f,-0.121638894f,0.198048726f,0.00917680189f,-0.138687477f,-0.0494632684f,0.233585089f,-0.183984488f,0.144229591f,0.0744586512f,0.0947782099f,0.142409325f,0.0921622142f,0.0682727024f,-0.0800848976f,-0.167953789f,0.173681632f,-0.0802400634f,0.315305889f,0.0536711961f,-0.0975886509f,0.0422310755f,-0.000621377316f,-0.106672801f,0.12361981f,0.0633782372f,-0.0301466975f,-0.142392084f,0.149335265f,-0.00126361777f,-0.333606809f,0.328839689f,0.11334794f,0.299254268f,-0.128882244f,-0.221274495f,-0.0269109327f,-0.104408346f,0.115575098f},{0.115311854f,0.0592520274f,-0.723284304f,-0.138764158f,0.167574763f,-0.727255225f,-0.31884867f,0.0222920608f,-0.122893102f,-0.0450593494f,0.194645762f,-0.0303554498f,0.0558783635f,0.0761784464f,-0.0859209299f,0.0162095092f,0.0543645062f,0.229153857f,-0.133942887f,0.105742842f,0.116761446f,-0.0550388582f,-0.176101461f,0.0356266499f,0.0650038645f,0.0231306702f,-0.082354635f,-0.137774333f,-0.281123608f,0.0253893286f,0.0159215797f,-0.0329078995f,0.399632603f,-0.0248308666f,-0.0554241687f,0.0174413528f,0.00153443275f,0.0264069289f,0.0840239003f,-0.0568605475f,-0.11816068f,-0.0204339735f,0.00499675609f,-0.244820371f,0.00801247824f,-0.0491211787f,0.215339676f,0.0167026669f,-0.186794057f,0.0441966206f,0.0156799499f,-0.0391235277f},{0.105427772f,-0.142806932f,-0.779893458f,0.114275023f,0.0181560609f,-0.838696957f,-0.383319736f,-0.0901056528f,-0.0878853947f,0.145927623f,0.176718816f,0.116831794f,0.00453041121f,0.0282346085f,0.0999539047f,-0.0553430729f,-0.00344753382f,-0.0548247211f,0.0833470225f,0.0419529453f,0.111440361f,0.0933981165f,0.0656271055f,0.0241568573f,0.0305645969f,-0.0225623548f,-0.089986749f,-0.106792398f,-0.147689417f,0.145395011f,-0.00669263536f,-0.141208604f,0.0972076505f,-0.00191414193f,0.0198108554f,0.0248843618f,-0.024325883f,0.0347307622f,-0.0419359393f,-0.000531467609f,-0.0181825832f,-0.0024431746f,-0.00709064305f,-0.125814527f,0.0256912801f,0.0390162393f,0.0196067803f,0.00878690369f,-0.0207084715f,-0.0300063025f,-0.0125524532f,0.013095255f},{0.039861884f,0.227426961f,0.132950634f,-0.271631271f,-0.191871598f,0.263912141f,-0.00919860508f,0.00362124946f,0.0933031142f,-0.0586338639f,-0.0589849055f,0.0359114818f,-0.188826039f,-0.257575363f,0.143206999f,0.00251423847f,-0.222298861f,0.0931054428f,0.19126758f,-0.324444801f,0.297396094f,-0.0287277922f,0.115033805f,0.0989414752f,-0.0174154826f,0.00475222897f,-0.232482657f,0.210062623f,-0.158074766f,-0.0641596168f,0.124917597f,-0.0346154496f,0.154481545f,0.0667032674f,0.0765653402f,-0.062446177f,-0.160098612f,-0.0297799949f,-0.100795247f,0.015560274f,-0.0403849445f,-0.0249215532f,0.417002171f,-0.270968884f,0.336341918f,-0.148951128f,0.182312995f,0.00559050217f,-0.0284035765f,0.192137584f,0.0339831263f,-0.0459228046f},{-0.287078261f,-0.0669651851f,0.220501244f,0.107291147f,0.106592864f,0.0969124734f,0.221749499f,-0.21990715f,-0.0283900127f,0.090830192f,0.0810341537f,0.207669571f,0.219791114f,0.275360137f,-0.221615091f,0.203494802f,-0.0105609242f,-0.203438044f,0.11441987f,0.0327603742f,-0.0430066884f,-0.0967173129f,0.0240189992f,0.0994397476f,0.160619274f,-0.132136092f,-0.14556855f,-0.0616556145f,-0.167803138f,0.139420107f,0.0727289468f,0.0564105399f,0.0622587577f,0.0309842452f,0.00121956097f,0.0133869462f,-0.0353335999f,0.0969508216f,-0.0255724117f,-0.0233747084f,-0.0365908779f,-0.444545239f,0.0090027852f,-0.244188502f,0.0516068712f,-0.0234500244f,-0.0702308863f,0.0647804067f,-0.12390326f,0.014134218f,-0.215100035f,0.080447644f},{0.0404671691f,0.0631401986f,-0.0219847709f,0.0408258885f,0.0897881165f,0.00893633906f,0.0112483213f,0.0193824116f,0.0557290912f,-0.0176131502f,-0.115833648f,-0.205388665f,-0.0427192748f,-0.0688731968f,0.108946823f,-0.0224953424f,-0.0370069034f,0.0602106452f,-0.145124406f,-0.0200267322f,0.0317408666f,-0.0581532121f,0.0187857263f,0.0491457433f,-0.0398406982f,0.0254187193f,0.0220366605f,-0.0550432838f,0.0575890914f,0.201610133f,0.0534921959f,0.100130543f,0.00740323309f,-0.0181779489f,0.0669201612f,-0.071165733f,0.0448746122f,0.0718822256f,-0.0775920227f,-0.0672838539f,0.0812837183f,0.121230431f,-0.066591531f,-0.274052531f,-0.0458666049f,0.132671714f,-0.0532149822f,0.0234562159f,-0.163672253f,0.109051473f,0.294055998f,-0.0782249421f},{0.0630884171f,0.275398374f,0.22639133f,-0.347777426f,-0.232496366f,0.131227821f,0.0138790654f,-0.140856907f,0.000851924298f,-0.0628240705f,-0.227058411f,-0.0682473108f,-0.294958442f,-0.371091604f,0.128703386f,-0.160179377f,-0.0376534276f,0.0880975723f,-0.158531025f,-0.359025925f,0.0641200915f,-0.0644176304f,0.119111076f,0.213494062f,0.309561342f,-0.0253299214f,-0.0495917909f,0.26971224f,0.0139011275f,0.0442958586f,-0.00863492675f,-0.245623574f,0.0821799114f,-0.124572635f,0.0582187772f,0.0279185548f,-0.0373047292f,-0.0894269794f,-0.00632436154f,0.0154392133f,0.0129531017f,0.0222755913f,0.366913706f,-0.195149302f,0.00901949313f,-0.0317037888f,-0.109026305f,0.00560098281f,-0.391579956f,0.272216588f,0.0281858239f,-0.0224618651f},{0.0457610413f,0.0201150868f,0.301161945f,-0.382995665f,-0.0251986049f,0.467468977f,0.358219445f,0.161380425f,0.166987002f,-0.113910742f,0.0615418032f,-0.0830249414f,0.0074631758f,-0.260733068f,0.302046776f,0.0949392021f,0.0829533264f,0.530976295f,0.165530711f,-0.222851411f,0.419503272f,0.241384298f,-0.0578905046f,0.000143841331f,0.279625416f,0.219100326f,0.20576638f,0.175375998f,-0.0373514704f,0.168073684f,-0.0600401014f,-0.407836795f,0.00387672824f,-0.004279071f,-0.0591716357f,0.0018731592f,-0.0648156703f,-0.00578431785f,0.0366751924f,-0.0414216854f,-0.054046534f,-0.104151011f,-0.129886076f,0.204666704f,0.0959035084f,-0.16985932f,-0.117741063f,0.0423768945f,-0.243868336f,0.245328739f,-0.030203484f,-0.046322275f},{-0.0614523962f,0.000544814742f,-0.0458415188f,-0.0165248066f,0.271663696f,0.00343842688f,0.316120058f,-0.0603215322f,-0.0722383857f,-0.0150696915f,0.349779993f,0.0764231384f,0.199398652f,0.0893710032f,-0.227163523f,-0.049424652f,0.0413066484f,-0.179522857f,0.126519442f,0.168246329f,-0.212844923f,0.131451353f,-0.166619092f,-0.0251336787f,0.197131917f,0.0390661247f,0.00273716985f,-0.222142071f,-0.110761657f,-0.0271295495f,-0.0640968159f,-0.0267106984f,0.0536970794f,0.0163930096f,0.0509614423f,-0.115806259f,-0.0296207797f,-0.00679404847f,0.0163085517f,0.132810131f,-0.00802373514f,-0.0326026641f,0.0478585362f,-0.361174136f,0.252400875f,0.0729672834f,-0.19905372f,-0.132769451f,0.132194579f,-0.10560748f,-0.201568797f,0.104506806f},{0.0956918597f,-0.19961302f,-0.351132631f,0.246058956f,0.223059148f,-0.364373147f,0.0905094445f,-0.113117002f,0.0556238964f,0.147852808f,0.298979938f,-0.00141090446f,-0.00805878267f,0.0395526662f,-0.237555876f,0.148245648f,-0.0501182936f,-0.388880342f,0.0899751857f,0.26641345f,-0.0797446817f,0.179054379f,-0.127905175f,-0.154598951f,0.00697812019f,-0.0102718147f,0.00836952869f,-0.0936395898f,-0.355961174f,0.105119288f,0.0775088966f,0.0861720368f,0.131826133f,-0.0977931023f,0.0509288907f,0.18157801f,-0.118144862f,0.173566207f,-0.034025915f,-0.0586477071f,0.076701507f,-0.0474726595f,-0.198779672f,-0.639352441f,0.117616571f,0.120180614f,-0.09228196f,0.203077048f,0.315667182f,-0.303518564f,0.0357393846f,0.0608396903f},{-0.00153999997f,0.101422153f,0.129345804f,-0.23724772f,-0.108635277f,0.149436563f,-0.163794562f,-0.0156890471f,0.00732962089f,-0.21636425f,-0.033594843f,-0.0360095426f,-0.152790621f,-0.0333939344f,-0.101107098f,-0.188473314f,-0.122106142f,0.128863513f,-0.155231938f,0.0445555598f,0.277762234f,-0.137489825f,0.378656179f,0.229805321f,0.035361588f,-0.00471026078f,0.0302486774f,0.197627887f,0.379025549f,-0.047240112f,0.0144822793f,-0.113054045f,0.189628676f,-0.00868925732f,0.130048439f,-0.115505815f,-0.078927815f,-0.0330798216f,0.00468738237f,-0.172944263f,-0.0010619954f,0.0195000488f,0.32003808f,0.587178946f,-0.000564725196f,-0.429664254f,-0.129307777f,0.0930510536f,-0.281944215f,0.275564164f,-0.0903275907f,0.0970762819f},{0.00558061572f,0.113532096f,-0.265173048f,0.212818488f,-0.0376205333f,-0.272562891f,-0.316052377f,0.239256397f,0.137102649f,0.0662788227f,0.0571372733f,-0.363892138f,-0.117651269f,0.0121535482f,0.286728412f,0.00301525206f,0.104238778f,0.507870078f,-0.216987938f,0.0142594362f,0.55991292f,-0.235735372f,0.0900114775f,0.309189439f,-0.258738488f,-0.0150636276f,0.0575794987f,-0.372281194f,-0.396160871f,0.329506487f,0.119572617f,0.041516494f,0.133932441f,0.0738714412f,0.0364756845f,-0.140407801f,0.0515180901f,0.160147727f,-0.0339553617f,0.0153285647f,0.0670749471f,-0.0535291806f,0.0178130642f,-0.761478841f,-0.0494475961f,0.0242617652f,-0.035039641f,0.00301903905f,0.126310199f,-0.0603739731f,-0.00152898347f,-0.038655974f},{-1.01382363f,-0.0306409579f,0.473195851f,0.00996359345f,0.0479051843f,0.669923544f,0.243361816f,0.130460337f,-0.084100619f,-0.0246119834f,0.23967059f,-0.0951204821f,-0.156473532f,-0.197100282f,0.0319826528f,-0.000247602584f,-0.220635414f,0.105955899f,-0.0408929624f,-0.0168946944f,0.0844627097f,0.056011457f,-0.168878451f,-0.0824172944f,0.0553025566f,0.0682953298f,0.238398269f,0.179577231f,0.117409267f,0.130393311f,-0.0924992338f,-0.047850281f,-0.292170882f,0.0765594468f,-0.0534104146f,-0.0438279249f,-0.0522406474f,0.0661195442f,-0.0222717617f,0.033369787f,0.0425313637f,-0.00541421724f,0.100896724f,0.417188674f,-0.121635459f,-0.0584088899f,0.0921786949f,0.418473631f,0.387904704f,0.0516904071f,-0.0221291203f,-0.0556267388f},{8.27506155e-05f,-0.207442492f,0.258997917f,0.0592605323f,0.054522533f,0.145541549f,0.113208406f,0.108621381f,0.0441500805f,-0.0936271474f,0.0205415972f,0.03572179f,-0.236138761f,-0.0336483419f,0.0361455046f,-0.250828505f,-0.173665017f,0.208967552f,-0.406869859f,-0.0717255324f,-0.0581686236f,-0.226275787f,-0.145560801f,-0.162942067f,0.341978073f,0.229027867f,-0.0573510714f,-0.12597248f,-0.499590874f,0.0583782494f,-0.0420167856f,0.164030463f,-0.14996241f,-0.00405548187f,0.102600574f,-0.0413173437f,-0.161250368f,-0.0759530962f,-0.072244674f,-0.0285267867f,-0.000700791134f,0.076266557f,0.270112008f,-0.185135409f,0.109348625f,-0.100626208f,0.0504983403f,0.0258300807f,0.363378465f,0.138728887f,-0.0602247491f,0.00831345376f},{0.132180244f,-0.0902271941f,-0.353733778f,0.371219695f,-0.0170226879f,-0.334040195f,-0.0460345373f,0.163800836f,0.0969466642f,0.0240260568f,-0.17381078f,0.0271564219f,-0.0554710627f,-0.0768121481f,0.249136597f,-0.0551776625f,0.193044156f,-0.166465744f,-0.104505442f,-0.0642435849f,0.0245808959f,-0.0811106712f,-0.0254661087f,0.109265961f,-0.165880054f,-0.0503781885f,0.000919632497f,-0.258658767f,-0.269314438f,0.137525305f,-0.00238866545f,0.78028065f,-0.0477905944f,0.0600845888f,-0.0675683469f,0.0515513159f,0.052010119f,0.0121172285f,-0.0458151139f,0.0537948608f,0.116707914f,-0.0108622219f,0.319061011f,-0.245109111f,0.176590577f,0.213490754f,0.162944496f,-0.0848134756f,0.44062072f,-0.302779019f,-0.0883164704f,0.101374567f},{0.0679179654f,-0.268839121f,-0.0870166719f,0.152309284f,0.027993897f,-0.0580354407f,0.0194118395f,0.0404566489f,0.107307568f,-0.0226360243f,0.140042812f,0.233979687f,0.194511265f,0.088522926f,-0.102391221f,0.0243349299f,-0.0838619471f,0.0631361827f,0.0338806212f,0.0696006194f,0.037067432f,-0.0152280107f,-0.208373159f,-0.262291431f,-0.0315490998f,-0.0744692981f,0.0376284048f,-0.187822953f,-0.185653731f,-0.00604816154f,0.0731135458f,-0.155567259f,0.0941489041f,0.0283456761f,0.00937437918f,0.0967759192f,-0.0503059998f,0.0401904322f,0.0358874314f,-0.0161411073f,-0.245876238f,-0.0133922044f,-0.211944044f,-0.33385247f,0.177398816f,0.0267587472f,0.072426036f,-0.152126685f,0.125806883f,-0.143498316f,-0.150655597f,-0.0127338283f},{-0.172597349f,0.121851794f,-0.105154909f,-0.202204749f,-0.165317073f,-0.0696400031f,-0.120918207f,0.103578933f,-0.080839321f,0.0558264554f,0.18846257f,-0.00192972284f,-0.237339869f,-0.221907079f,0.183024168f,-0.0816078559f,-0.0654259622f,0.0670865402f,0.0888820961f,-0.114086971f,0.233568043f,0.0371143781f,-0.152000785f,0.155902743f,-0.140999809f,0.0533563308f,0.0553729869f,0.150673136f,0.0427843221f,0.255253732f,0.16305083f,0.0085812714f,0.0159943476f,-0.1223673f,-0.114384115f,0.0653822571f,-0.026605757f,0.0548842959f,-0.0509199277f,-0.0533958934f,-0.152486131f,0.0335125849f,0.0556356087f,0.584147632f,-0.382691056f,-0.104962029f,0.146326348f,0.153485477f,0.000239923873f,-0.00614635553f,-0.0105173793f,0.0486115962f},{0.144342184f,-0.126422644f,-0.248143017f,-0.102682754f,0.0354548506f,-0.211826757f,-0.0404416174f,-0.0232098736f,0.0079232445f,-0.159038067f,-0.177459985f,0.0366146117f,-0.00789619703f,-0.200122461f,-0.0533424057f,0.0221473444f,0.0659571961f,0.216158554f,0.219758213f,0.160258278f,0.014776947f,0.301463872f,-0.0594431534f,-0.0627016351f,0.040766716f,0.269002855f,0.272027522f,0.180886775f,-0.0983357877f,0.424772859f,-0.13496086f,0.00947886985f,-0.00603287015f,-0.1605317f,-0.0338801593f,-0.0584406033f,0.0830622986f,0.062279433f,-0.0225098711f,0.0622268803f,-0.0136091001f,-0.0539991185f,-0.0259051118f,0.119808167f,0.182677045f,0.100991577f,0.274769664f,-0.00179116207f,-0.432057589f,-0.0723380521f,-0.0427535251f,-0.0257266462f},{-0.0125541622f,0.247581005f,0.200394735f,-0.286681592f,-0.220749214f,0.163425863f,-0.170356825f,-0.0478364937f,-0.0744013265f,-0.228698835f,0.114567779f,-0.0461502261f,-0.0855090469f,-0.210521296f,-0.146322593f,-0.00761468895f,-0.0204928312f,0.12861751f,-0.21939759f,-0.242049068f,0.320787191f,-0.172292292f,0.228278771f,0.178601697f,0.0631665662f,-0.0872938931f,0.308858484f,0.272286147f,0.522175789f,-0.022306392f,0.026274845f,-0.103627279f,-0.0293763019f,0.133466348f,-0.0413860232f,0.0218450893f,-0.0683754608f,-0.11590793f,-0.0423813015f,-0.0847750828f,-0.0455298014f,0.0530600622f,0.316972077f,0.594817102f,-0.0942783654f,-0.335655212f,-0.153116852f,-0.0612938255f,-0.283257097f,0.470247388f,-0.0660736486f,-0.000293186167f},{-0.111150786f,-0.2812953f,0.146944806f,0.0816583708f,0.076024726f,0.0762181282f,0.200687394f,0.222463548f,-0.0235832557f,-0.0589591302f,0.104900621f,0.00969185866f,0.130151376f,0.156849876f,0.13864626f,-0.0137016652f,-0.0310874134f,-0.00972059276f,-0.109422669f,-0.0688837394f,0.0533361658f,-0.108491749f,-0.274083108f,-0.0708111376f,0.0525993891f,-0.078663975f,-0.052043803f,-0.154278085f,-0.224248797f,0.11540702f,-0.03593174f,-0.315306723f,-0.309480518f,-0.0162608605f,-0.0316724777f,0.0565945692f,-0.020635739f,-0.0264351368f,0.134990945f,0.0880181938f,-0.205209613f,-0.113513432f,0.0138483252f,-0.513032973f,0.288541734f,0.0623870343f,0.00564142317f,0.196167722f,0.003935616f,-0.241548181f,-0.16899839f,0.0284091104f},{0.151460692f,0.215443283f,0.327127546f,-0.31933406f,0.00393229956f,0.279699624f,-0.0293519963f,-0.0970578194f,-0.00292611145f,0.0711230263f,-0.433556318f,0.0561796799f,-0.0884129182f,-0.0825961381f,0.068206884f,0.0294774547f,-0.0303107332f,0.120856993f,-0.208611369f,-0.228141651f,0.135466442f,-0.226966783f,0.0757352561f,-0.0388934538f,0.117762327f,-0.0707256868f,0.043244604f,0.300664395f,0.0246831011f,0.0414007455f,-0.126082495f,-0.45233053f,0.159994677f,-0.0525160804f,0.0470113792f,0.182144254f,-0.192597285f,-0.146756828f,-0.105288185f,-0.0392132774f,-0.0817549974f,0.110586442f,0.113140099f,0.0200224146f,0.102110215f,-0.225880876f,-0.139493361f,-0.129458264f,-0.413886935f,0.478877157f,-0.02527369f,0.00709218206f},{0.0435597152f,-0.0298036635f,0.17784968f,-0.0979939103f,0.0502021164f,0.185899183f,0.284755975f,0.02687243f,-0.00274930475f,0.0970114246f,0.00717792567f,-0.135053605f,0.0539510809f,0.0473842584f,0.133952543f,-0.0236453488f,-0.127640858f,0.0546273105f,-0.11991214f,0.0870362371f,0.0734525397f,-0.0873512328f,0.206740096f,-0.09363015f,0.0113156438f,-0.0590449125f,-0.0368233845f,-0.335334569f,0.00136389013f,0.0805818513f,0.0348043889f,0.0685429871f,0.118083797f,-0.00507994602f,0.078861095f,0.0608491004f,-0.066690594f,0.139310971f,-0.13558732f,-0.0145363091f,-0.00302429893f,0.0159128048f,0.0358371027f,-0.890755713f,-0.149953336f,0.0829716176f,-0.347751439f,-0.120198488f,-0.0259900261f,-0.0440769158f,0.0943558514f,-0.0842034221f},{-0.00284959143f,-0.129604563f,-0.0139708025f,0.00896726269f,0.0529707968f,0.034458831f,-0.128192171f,0.030720761f,0.0644977912f,-0.113334246f,0.185130864f,-0.122858904f,-0.0834577382f,-0.191516802f,-0.10778974f,-0.191145688f,-0.213764325f,0.0847442895f,-0.249627501f,0.00259489822f,0.260791838f,-0.279748857f,0.329471141f,0.016097419f,-0.0941665545f,-0.164185539f,0.136477336f,0.338939875f,0.403405845f,-0.0761317685f,-0.0131378165f,-0.502534568f,-0.203236058f,0.0202729646f,0.0498839691f,0.0153108286f,-0.0710103437f,-0.0755077526f,0.0104894266f,-0.046151191f,-0.00685920846f,0.0311535187f,0.174436256f,0.492258966f,-0.105509736f,0.0129142739f,-0.159651667f,-0.0425090864f,0.135518804f,0.0699926317f,0.0804233551f,0.0150886849f}};
 static constexpr float intercept[]={-0.254773527f,-0.114183918f,1.14466119f,0.224979594f,-0.331987172f,-0.364689738f,-0.112334453f,-0.276653767f,-0.89689064f,0.173727214f,-0.53014797f,-1.34743953f,-0.592527449f,0.354057223f,0.0743788555f,-0.597432435f,-0.24981086f,-0.0883304253f,0.386196434f,-0.32550332f,0.0442030616f,0.999598682f,-0.194730863f,0.0835359395f,0.124674819f,0.228218481f,-0.272190601f,-0.587832451f,-0.0398570523f,-0.0111561082f,0.0900204778f,-0.849972665f},out[]={-0.11585696f,0.100469388f,0.362995654f,0.114872396f,-0.0689164251f,-0.145622164f,0.141525939f,-0.201297864f,0.61308825f,0.114245623f,0.116326638f,0.748445451f,-0.178598449f,0.121228166f,0.130767614f,-0.19794637f,-0.229795486f,0.0975667983f,0.142870277f,0.208689466f,0.330243796f,0.243409202f,-0.19402273f,-0.149741486f,0.0547987036f,0.0706925169f,-0.298465073f,0.320928812f,0.0792173445f,-0.206118554f,0.108354814f,0.30821085f};
 double total=0;int product=0;for(const auto& r:x){float z[geometry_price::width],v=0;
  for(int j=0;j<geometry_price::width;++j){z[j]=(r[j]-mu[j])*inv[j];v+=weight[j]*z[j];}
  for(int t=0;t<32;++t){float q=intercept[t];for(int j=0;j<geometry_price::width;++j)q+=hidden[t][j]*z[j];v+=out[t]*max(0.f,q);}
  items[product++]=v;total+=v;}return 4*total;
}
}

// A fitted complementary family extends to a new axis by taking the orientation
// needed by its paired clients. Positions left undecided by the clients keep
// their old value. The other products still compete for these donors.
struct AxisProposal {
 array<int,N> map{},repair{};
 array<U,MaxGroups> codes{};
 array<U,P> targets{};
 int groups=0;
 void observe(){
  map=bitMap;groups=DonorGroups;copy_n(A,P,targets.begin());repair.fill(0);
  int votes[MaxGroups][N][2]{};
  for(auto& p:patches)if(p.c[0]>=P){
   int b=(p.c[0]-P)/2,orient=(p.c[0]-P)%2;U delta=A[p.a[0]]^A[p.a[1]];
   for(;delta;delta&=delta-1){int z=ctz(delta);votes[b][z][(A[p.a[orient]]>>z)&1]++;}
  }
  for(int b=0;b<groups;b++){
   codes[b]=A[P+2*b];
   for(int z=0;z<N;z++)if(votes[b][z][1]!=votes[b][z][0]){
    codes[b]&=~(U(1)<<z);codes[b]|=U(votes[b][z][1]>votes[b][z][0])<<z;
   }
  }
  for(int a=0;a<P;a++){
   U covered=0;for(int k:core[a])covered|=cards[k].m;
   int weight=1+(sampledProbe[a][0]>=13);
   for(U holes=~covered;holes;holes&=holes-1)repair[ctz(holes)]+=weight;
  }
 }
 array<int,N> construct(int seedWidth){
  struct Edge{int a,b,rank,total;U delta;unsigned forward=0,reverse=0;};
  vector<Edge> edges;
  U head=0;for(int i=0;i<seedWidth;i++)head|=U(1)<<(i*32/seedWidth);
  for(int a=0;a<P;a++)for(int b=0;b<a;b++){
   U d=targets[a]^targets[b];edges.push_back({a,b,0,pc(d),d});
  }
  constexpr int shortlist=3072;
  partial_sort(edges.begin(),edges.begin()+shortlist,edges.end(),[&](const Edge& a,const Edge& b){
   auto score=[&](const auto& e){return 3*e.total+pc(e.delta&~head);};
   return tuple(score(a),a.a,a.b)<tuple(score(b),b.a,b.b);
  });
  edges.resize(shortlist);auto words=codes;
  for(int step=seedWidth;step<32;step++){
   for(auto& e:edges){
    e.forward=e.reverse=0;U d=e.delta&head;
    for(int g=0;g<groups;g++){U wrong=(targets[e.a]^words[g])&d;
     if(!wrong)e.forward|=1u<<g;if(wrong==d)e.reverse|=1u<<g;}
    e.rank=4*(22+(32-step)/3-pc(e.delta&~head))-e.total/3;
   }
   // Once a pair has no orientation on the current head, appending another
   // coordinate cannot recover it. New cliques are generated after growth.
   erase_if(edges,[](const Edge& e){return !(e.forward|e.reverse);});
   sort(edges.begin(),edges.end(),[](const Edge& a,const Edge& b){return tie(a.rank,a.a,a.b)>tie(b.rank,b.a,b.b);});
   vector<int> usedRows;
   auto pack=[&](U bit,unsigned signs,bool remember){
    array<bool,P> taken{};int count=0,gain=0;
    for(int i=0;i<int(edges.size());i++){
     auto& e=edges[i];if(e.rank<=-4)break;if(taken[e.a]||taken[e.b])continue;
     bool diff=(e.delta&bit)!=0;
     if(diff){unsigned agree=(targets[e.a]&bit)?signs:~signs;
      if(!(e.forward&agree)&&!(e.reverse&~agree))continue;}
     int value=e.rank+4*diff;if(value<=0)continue;
     taken[e.a]=taken[e.b]=true;gain+=value;
     if(remember)usedRows.push_back(i);if(++count==P-K+2*groups)break;
    }
    return gain;
   };
   pack(0,0,true);
   struct Choice{int score,z;unsigned signs;};vector<Choice> choices;
   for(U rem=~head;rem;rem&=rem-1){
    int z=ctz(rem);U bit=U(1)<<z;int vote[MaxGroups]{};
    for(int i:usedRows){auto& e=edges[i];if(!(e.delta&bit))continue;
     int w=(targets[e.a]&bit)?1:-1;
     w*=max(1,e.rank/max(1,popcount(e.forward|e.reverse)));
     for(unsigned m=e.forward&~e.reverse;m;m&=m-1)vote[countr_zero(m)]+=w;
     for(unsigned m=e.reverse&~e.forward;m;m&=m-1)vote[countr_zero(m)]-=w;
    }
    unsigned signs=0;
    for(int g=0;g<groups;g++)if(vote[g]>0||(!vote[g]&&(words[g]&bit)))signs|=1u<<g;
    choices.push_back({pack(bit,signs,false),z,signs});
   }
   stable_sort(choices.begin(),choices.end(),[](auto a,auto b){return a.score>b.score;});
   Choice best=choices[0];
   for(int i=0;i<4;i++){
    auto trial=choices[i];U bit=U(1)<<trial.z;
    for(int g=0;g<groups;g++){
     unsigned signs=trial.signs^(1u<<g);int v=pack(bit,signs,false);
     if(v>trial.score){trial.score=v;trial.signs=signs;}
    }
    if(trial.score>best.score)best=trial;
   }
   U bit=U(1)<<best.z;head|=bit;
   for(int g=0;g<groups;g++)words[g]=(words[g]&~bit)|((best.signs>>g&1)?bit:0);
  }
  array<int,N> order{},indices{};iota(indices.begin(),indices.end(),0);
  stable_partition(indices.begin(),indices.end(),[&](int z){return (head>>z)&1;});
  for(int j=0;j<N;j++)order[j]=map[indices[j]];
  inheritedCount=groups;
  for(int b=0;b<groups;b++){
   U target=0;for(int j=0;j<32;j++)target|=((words[b]>>indices[j])&1ULL)<<j;
   inheritedCodes[b]=target;
  }
  return order;
 }

};
#define BOUNDED_FEATURES
// v5024_live_candidates.hpp
namespace bounded_design {
struct Survey {
 array<U,K> mask{},ink{};array<uint8_t,K> grip{};
 array<array<int16_t,2>,P> anchor{};array<int16_t,P> partner{};array<float,P> item{};
};
struct Candidate {
 bool valid=false;int slot=-1,trial=0,groups=0,inherit=0,mode=0;
 array<int,N> order{};array<U,MaxGroups> words{};
 DesignSnap design;array<uint8_t,K> shelf{};shared_ptr<ShelfRequests> requests;
 array<int16_t,P> partner{},backer{};array<float,P> item{};
 double price=0;shared_ptr<Survey> survey;
#if defined(BOUNDED_FEATURES) || defined(BOUNDED_AUDIT)
 geometry_price::Features features{};array<array<float,8>,P> repairs{};
#endif
};
array<Candidate,6> parents;
U physical(U w,const array<int,N>& map){
 U out=0;while(w){int z=ctz(w);out|=U(1)<<map[z];w&=w-1;}return out;
}
void record(Candidate& c,int mode,int id,const array<uint8_t,K>& shelf,const shared_ptr<ShelfRequests>& requests,
            const geometry_price::Features& xf,const array<float,P>& item,double price){
 c.valid=true;c.mode=mode;c.slot=id;c.shelf=shelf;c.requests=requests;c.item=item;c.price=price;c.design.save();
 c.partner.fill(-1);c.backer.fill(-1);
 for(const auto& p:patches)for(int j=0;j<2;++j){c.partner[p.a[j]]=p.a[j^1];c.backer[p.a[j]]=p.c[j];}
#if defined(BOUNDED_FEATURES) || defined(BOUNDED_AUDIT)
 c.features=xf;
 for(int a=0;a<P;++a){
  U covered=0;for(int k:core[a])covered|=cards[k].m&~(cards[k].v^A[a]);U missing=~covered;
  int best=64,withHead=64,whole=0,options=0;
  for(int k=0;k<K;++k){
   if(find(core[a].begin(),core[a].end(),k)!=core[a].end())continue;
   U after=missing&~(cards[k].m&~(cards[k].v^A[a]));
   best=min(best,pc(after));whole+=after==0;
   if(!(after&low)){withHead=min(withHead,pc(after));++options;}
  }
  c.repairs[a]={float(core[a].size()),float(c.backer[a]>=P),float(pc(missing&low)),float(pc(missing&~low)),
                float(best),float(withHead),float(whole),float(options)};
 }
#endif
#ifdef BOUNDED_AUDIT
 if(const char* path=getenv("BOUNDED_DUMP")){
  string name=string(path)+"/"+to_string(10*mode+id);
  ofstream f(name+".features",ios::binary);
  f.write(reinterpret_cast<const char*>(&c.price),sizeof(c.price));
  f.write(reinterpret_cast<const char*>(&c.features),sizeof(c.features));
  f.write(reinterpret_cast<const char*>(&c.repairs),sizeof(c.repairs));
  f.close();if(!f)abort();
  auto stack=shelf;crossEndpointShelf(stack,requests->synthetic);
  ofstream output(name+".paper",ios::binary);
  for(int k:stack){string row(N,'.');for(int z=0;z<N;++z)if(cards[k].m>>z&1)row[bitMap[z]]='0'+(cards[k].v>>z&1);output<<row<<'\n';}
  output.close();if(!output)abort();
 }
#endif
}
void observe(Candidate& p){
 if(!p.survey){
  p.survey=make_shared<Survey>();auto& s=*p.survey;
  p.design.load();auto stack=p.shelf;crossEndpointShelf(stack,p.requests->synthetic);
  array<int,N> level{};
  for(int k:stack){int d=N;for(U w=cards[k].m;w;w&=w-1){int z=ctz(w);d=min(d,level[z]++);}s.grip[k]=d;}
  for(int k=0;k<K;++k){
   s.mask[k]=observed_sheets::remap(physical(cards[k].m,bitMap),p.order);
   s.ink[k]=observed_sheets::remap(physical(cards[k].m&cards[k].v,bitMap),p.order);
  }
  double mean=accumulate(p.item.begin(),p.item.end(),0.)/P;
  for(int a=0;a<P;++a){
   s.anchor[a]={int16_t(core[a][0]),int16_t(core[a].size()>1?core[a][1]:-1)};
   s.item[a]=float(p.item[a]-mean);
  }
  s.partner=p.partner;
 }
 const auto& s=*p.survey;observed_sheets::mask=s.mask;observed_sheets::ink=s.ink;observed_sheets::grip=s.grip;
 observed_sheets::anchor=s.anchor;observed_sheets::partner=s.partner;observed_sheets::item=s.item;
}
struct Proposal{int mode,parent;};
// v5025_learned_rank.hpp
constexpr int aggregateSize=131,inputWidth=166;
using Aggregate=array<double,aggregateSize>;
using Input=array<float,inputWidth>;
double feature(const Candidate& c,int a,int j){return j<52?c.features[a][j]:c.repairs[a][j-52];}
Aggregate aggregate(const Candidate& c){
 Aggregate out{};array<double,60> sq{};out[0]=c.price/1024.;
 constexpr int mixed[]={0,3,23,42,43,46,54,55,56,57};
 for(int a=0;a<P;++a){
  for(int j=0;j<60;++j){double x=feature(c,a,j);out[1+j]+=x/P;sq[j]+=x*x/P;}
  if(c.backer[a]>=P)for(int j=0;j<10;++j)out[121+j]+=feature(c,a,mixed[j])/P;
 }
 for(int j=0;j<60;++j)out[61+j]=sqrt(max(0.,sq[j]-out[1+j]*out[1+j]));
 return out;
}
Input compareFeatures(const Candidate& c,const Candidate& ref,int mode,const Aggregate& a,const Aggregate& b){
 Input z{};for(int j=0;j<aggregateSize;++j)z[j]=float(a[j]-b[j]);
 int pos=aggregateSize;
 if(mode)z[pos+(mode>=100?mode-101+8:mode-1)]=1;pos+=16;
 array<double,8> local{};
 for(int i=0;i<P;++i){
  bool changed=c.partner[i]!=ref.partner[i];
  local[0]+=changed;local[1]+=(c.backer[i]>=P)!=(ref.backer[i]>=P);
  if(!changed)continue;
  local[2]+=double(c.item[i])-ref.item[i];
  local[3]+=double(c.features[i][42])-ref.features[i][42];local[4]+=double(c.features[i][43])-ref.features[i][43];
  for(int k=0;k<3;++k)local[5+k]+=double(c.repairs[i][2+k])-ref.repairs[i][2+k];
 }
 for(double x:local)z[pos++]=float(x/P);
 constexpr int absCol[]={0,43,44,55,56,57,58,60};
 for(int j:absCol)z[pos++]=mode?float(a[j]):0;
 z[pos++]=int(c.slot==1)-int(ref.slot==1);z[pos++]=int(c.slot>=4)-int(ref.slot>=4);z[pos++]=float(c.slot-ref.slot)/5.f;
 return z;
}

constexpr int budgetLimit=8;
constexpr int usedModes[]={ 8,105,1,108,2,104,4,101 };
double preModel(const Input& z){
static constexpr float inv[]={1.688203430e+01f,5.000000000e+02f,3.752842712e+01f,2.609253693e+01f,4.607584953e+00f,5.014654160e+00f,2.445788193e+01f,2.017129135e+01f,1.978912735e+01f,1.340887737e+01f,5.508375168e+00f,3.159387398e+01f,2.552294922e+01f,1.163832855e+01f,4.886501789e+00f,2.403899574e+01f,8.755432129e+00f,5.088659763e+00f,2.331586838e+01f,8.895023465e-01f,4.716422558e+00f,1.896372604e+01f,8.895023465e-01f,3.171373940e+01f,3.097607994e+01f,3.910051823e+00f,3.880245686e+00f,3.328332663e+00f,3.057078600e+00f,3.478195667e+00f,3.751662493e+00f,5.685601711e+00f,6.296712112e+01f,3.789940643e+01f,2.341986418e+00f,2.480933905e+00f,2.277813196e+00f,2.130986929e+00f,1.482253432e+00f,2.322792053e+00f,2.791547775e+00f,3.759024811e+01f,5.000000000e+02f,4.596978188e+00f,6.211314678e+00f,3.212318182e+00f,9.928486824e+00f,4.884806871e-01f,2.502482605e+02f,3.704306030e+01f,4.607584953e+00f,1.775736734e-02f,2.193236113e+00f,1.403671112e+02f,2.892779541e+01f,1.445292568e+01f,5.248636723e+00f,9.513918877e+00f,9.461981773e+00f,1.936484694e+00f,1.779662669e-01f,5.000000000e+02f,3.352310562e+01f,4.950459290e+01f,3.843574047e+00f,4.033267975e+00f,5.161650848e+01f,3.187618637e+01f,1.712626457e+01f,7.785555363e+00f,3.681999683e+00f,3.469853973e+01f,1.871773720e+01f,8.094413757e+00f,3.698457003e+00f,1.774564171e+01f,5.381842136e+00f,4.158081055e+00f,6.141220093e+01f,2.080694675e+00f,3.964836121e+00f,2.595396233e+01f,2.080694675e+00f,3.426592255e+01f,3.426137924e+01f,1.501172829e+01f,1.545313454e+01f,6.751024246e+00f,4.167005539e+00f,3.965869904e+00f,1.329567814e+01f,8.144856453e+00f,5.771940231e+01f,3.413634872e+01f,3.583685160e+00f,3.795114994e+00f,3.796762228e+00f,3.523839712e+00f,1.192721605e+00f,3.940437794e+00f,4.395392895e+00f,3.498771667e+01f,9.664363098e+01f,2.634153843e+00f,4.443278790e+00f,4.179647923e+00f,1.402225494e+01f,4.419099689e-01f,2.170363617e+01f,2.239527702e+01f,3.843574047e+00f,5.000000000e+02f,5.000000000e+02f,5.000000000e+02f,3.002986755e+02f,6.870995998e+00f,4.644913673e+00f,9.844413757e+00f,9.125439644e+00f,6.982394457e-01f,1.157609597e-01f,4.173398590e+01f,9.992597699e-01f,4.338012314e+01f,2.510884285e+00f,6.524584293e+00f,1.634460837e-01f,3.765904617e+01f,1.577747583e+00f,4.979220390e+00f,4.959312916e+00f,2.834569693e+00f,2.823924065e+00f,5.000000000e+02f,2.827065706e+00f,5.000000000e+02f,5.000000000e+02f,5.000000000e+02f,2.824185371e+00f,2.824446917e+00f,5.000000000e+02f,5.000000000e+02f,2.826279402e+00f,2.830349684e+00f,5.000000000e+02f,5.000000000e+02f,2.836686850e+00f,1.586789370e+00f,2.617372990e+00f,1.140009403e+01f,5.309121609e+00f,7.455682278e+00f,1.865463638e+01f,3.606134653e+00f,8.757701874e+00f,1.531880140e+00f,8.559545130e-02f,3.055089116e-01f,6.290827274e+00f,7.949206978e-02f,2.794063687e-01f,2.769947648e-01f,5.645697471e-03f,2.193236113e+00f,1.419786215e+00f,2.015811443e+00f};
static constexpr float w[]={-6.624534726e-01f,0.000000000e+00f,-2.190588564e-01f,-4.506038502e-02f,-1.283358410e-02f,-2.309888415e-02f,-3.098415956e-02f,9.614900500e-02f,-2.093015760e-01f,-2.947124280e-02f,1.479561329e-01f,-6.947398931e-02f,1.382994745e-02f,-1.037935987e-01f,7.471040636e-02f,3.064954877e-01f,8.665451407e-02f,-1.625432819e-02f,6.282695383e-02f,5.238661543e-02f,-1.025123447e-01f,-1.011632383e-02f,5.238661543e-02f,1.731437445e-02f,6.430551410e-02f,5.528977141e-02f,-4.331481457e-02f,-6.562801544e-03f,4.448302463e-02f,-1.803422347e-02f,3.227045527e-04f,-9.528698027e-02f,4.136882257e-03f,1.661549658e-01f,4.134319723e-03f,5.828191712e-02f,6.475509703e-02f,-7.422778010e-02f,-2.264972281e-04f,-1.149825081e-01f,2.640836313e-02f,2.797954716e-02f,-3.920666128e-02f,-5.293123797e-02f,4.061651975e-02f,9.310209006e-02f,5.629241839e-02f,6.369788945e-02f,-1.673767343e-02f,2.355252802e-01f,1.283358410e-02f,5.920190364e-02f,3.534181044e-02f,3.534181044e-02f,-1.693602651e-01f,-4.128358141e-02f,2.639225423e-01f,3.756185248e-02f,-4.589461535e-02f,-1.782774329e-01f,-2.534252703e-01f,0.000000000e+00f,1.468043178e-01f,5.651506037e-02f,2.392640524e-02f,1.624245569e-02f,-8.526485413e-02f,6.738793850e-02f,-9.350912645e-03f,-3.457055986e-02f,-5.436879024e-02f,-5.759191513e-02f,1.760857925e-02f,9.767286479e-02f,6.725353748e-02f,5.246859416e-02f,2.349956706e-02f,5.002789199e-02f,2.989699692e-02f,-1.452236995e-02f,-1.497547477e-01f,5.140576512e-02f,-1.452236995e-02f,8.491562679e-03f,-4.149223212e-03f,-5.933346972e-02f,5.616512615e-03f,-2.005179971e-01f,2.190477848e-01f,3.320419416e-02f,2.495044097e-02f,2.563263755e-03f,-5.661009345e-03f,-1.439765692e-01f,-8.582171798e-02f,9.303756058e-02f,-2.924006246e-02f,4.096024111e-02f,1.789301261e-02f,-3.030323423e-03f,2.449868433e-02f,-2.237546258e-02f,2.067552321e-02f,-9.583136067e-03f,-1.340767834e-02f,1.522952504e-02f,-1.703812857e-03f,2.565458231e-02f,-5.720163230e-03f,2.461756021e-02f,2.392640524e-02f,0.000000000e+00f,0.000000000e+00f,-2.611945011e-02f,-6.145899370e-02f,1.951374672e-02f,-1.331929862e-01f,-1.698742434e-02f,5.995621532e-02f,1.827867329e-01f,-7.486969233e-02f,-1.693602353e-01f,-2.023188472e-01f,-7.851953059e-02f,9.242881089e-02f,-4.911998287e-02f,-1.419394165e-01f,-6.691124290e-02f,3.743042946e-01f,-1.600840315e-02f,-8.268138394e-03f,1.220959127e-01f,6.310460716e-02f,0.000000000e+00f,6.168035418e-02f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,6.814872473e-02f,9.722311050e-02f,0.000000000e+00f,0.000000000e+00f,9.528518468e-02f,1.136681214e-01f,0.000000000e+00f,0.000000000e+00f,1.380108893e-01f,6.119064987e-02f,-7.470750809e-02f,-9.009863250e-03f,5.929657817e-02f,-4.316459224e-02f,2.033213852e-03f,3.423884278e-03f,-3.676672652e-02f,-1.623725891e+00f,-1.014293358e-02f,1.939432174e-01f,1.577781588e-01f,2.614910901e-01f,3.353737593e-01f,3.387700915e-01f,2.518473268e-01f,-3.534181044e-02f,-1.108691562e-02f,3.901617602e-03f};
Input x;float v=0;
for(int j=0;j<inputWidth;++j){x[j]=clamp(z[j]*inv[j],-8.f,8.f);v+=w[j]*x[j];}
return 64.*v;}
array<Aggregate,6> parentAgg;
vector<Proposal> propose(int selected){
 const auto& ref=parents[selected];vector<pair<double,Proposal>> all;
 for(int i=0;i<6;++i)if(parents[i].valid)parentAgg[i]=aggregate(parents[i]);
#ifdef BOUNDED_AUDIT
 ofstream audit;
 if(const char* p=getenv("BOUNDED_DUMP"))audit.open(string(p)+"/pre.tsv");
#endif
 for(int mode:usedModes)for(int i=0;i<6;++i)if(parents[i].valid){
  const auto& c=parents[i];auto x=compareFeatures(c,ref,mode,parentAgg[i],parentAgg[selected]);
  double score=c.price-ref.price+preModel(x);
  all.push_back({score,{mode,i}});
#ifdef BOUNDED_AUDIT
  if(audit)audit<<mode<<" "<<i<<" "<<setprecision(17)<<score<<'\n';
#endif
 }
 stable_sort(all.begin(),all.end(),[](const auto& x,const auto& y){
  return tie(x.first,x.second.mode,x.second.parent)<tie(y.first,y.second.mode,y.second.parent);
 });
 vector<Proposal> res;
 for(const auto& p:all){
  res.push_back(p.second);
#ifdef BOUNDED_AUDIT
  if(getenv("BOUNDED_ALL"))continue;
#endif
  if(int(res.size())>=budgetLimit)break;
 }
 return res;
}
// v5048_duel_choice.hpp
constexpr double duelMix=1.0,duelThreshold=6.0;
double duelModel(const Input& z){
static constexpr float inv[]={1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,6.849906445e+00f,4.602677345e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,5.329798698e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,4.601130962e+00f,1.000000000e+01f,8.677155495e+00f,5.202033043e+00f,1.000000000e+01f,1.070147276e+00f,4.251191139e+00f,1.000000000e+01f,1.070147276e+00f,1.000000000e+01f,1.000000000e+01f,3.965372562e+00f,4.015692711e+00f,3.500069618e+00f,3.156316519e+00f,3.370265961e+00f,3.907317400e+00f,6.194064140e+00f,1.000000000e+01f,1.000000000e+01f,2.169627666e+00f,2.286911249e+00f,2.164305449e+00f,2.067648888e+00f,1.501393676e+00f,2.210025549e+00f,2.833472967e+00f,1.000000000e+01f,1.000000000e+01f,6.043531418e+00f,6.927391052e+00f,3.097142935e+00f,1.000000000e+01f,4.892034233e-01f,1.000000000e+01f,1.000000000e+01f,6.849906445e+00f,2.218110114e-02f,4.461905479e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,6.044052601e+00f,9.107382774e+00f,8.985557556e+00f,1.908110261e+00f,1.848098487e-01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,4.479819298e+00f,4.449791908e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,8.603634834e+00f,4.157030106e+00f,1.000000000e+01f,1.000000000e+01f,9.059050560e+00f,4.139384270e+00f,1.000000000e+01f,6.044935703e+00f,4.731097698e+00f,1.000000000e+01f,2.531977415e+00f,4.389706612e+00f,1.000000000e+01f,2.531977415e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,6.490330696e+00f,4.371733665e+00f,4.364333630e+00f,1.000000000e+01f,9.248508453e+00f,1.000000000e+01f,1.000000000e+01f,2.585624456e+00f,2.676533222e+00f,2.703296185e+00f,2.637273788e+00f,1.213339925e+00f,2.760759354e+00f,4.610922337e+00f,1.000000000e+01f,1.000000000e+01f,3.400686264e+00f,6.109251022e+00f,4.084884167e+00f,1.000000000e+01f,4.335715473e-01f,1.000000000e+01f,1.000000000e+01f,4.479819298e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,5.500340462e+00f,1.000000000e+01f,9.203729630e+00f,6.933525205e-01f,1.547888666e-01f,1.000000000e+01f,1.237244964e+00f,1.000000000e+01f,2.671132326e+00f,8.426975250e+00f,1.974743903e-01f,1.000000000e+01f,1.913071513e+00f,5.468158245e+00f,5.223673820e+00f,1.000000000e+01f,2.423451424e+00f,1.000000000e+01f,2.157190561e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,2.465858698e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,2.498903751e+00f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,1.000000000e+01f,2.662499189e+00f,4.419101715e+00f,1.000000000e+01f,9.123682976e+00f,1.000000000e+01f,1.000000000e+01f,4.507092476e+00f,1.000000000e+01f,1.902981758e+00f,1.024058834e-01f,3.729045093e-01f,7.579185963e+00f,9.408538789e-02f,3.315042555e-01f,3.287817240e-01f,6.762833335e-03f,4.461905479e+00f,2.467885733e+00f,3.441452265e+00f};
static constexpr float w[]={-3.505853713e-01f,0.000000000e+00f,4.148698226e-02f,-5.563941784e-03f,7.808897179e-03f,3.230522200e-02f,-8.268280886e-03f,1.893891580e-02f,-3.224376962e-02f,1.376450434e-02f,1.159191225e-02f,2.252476290e-02f,6.397100538e-02f,-2.173768543e-02f,-1.686976478e-02f,-2.223263308e-02f,-9.456313215e-03f,-1.987946220e-02f,1.620756648e-02f,-2.212348953e-02f,1.996715553e-02f,-4.463401064e-02f,-2.212348953e-02f,2.822869457e-02f,7.059606910e-02f,2.909956127e-02f,-7.278262638e-04f,7.556825411e-03f,-6.312720478e-02f,-4.959813505e-02f,2.564076148e-02f,-2.337880805e-02f,1.793366112e-02f,5.603769422e-02f,1.574246958e-02f,-2.199668065e-02f,-1.849767379e-02f,-5.637697503e-02f,3.602706641e-02f,1.516563818e-02f,-5.285062268e-02f,6.483868510e-02f,-5.599446222e-02f,1.858570613e-02f,-1.236792095e-02f,-4.664591700e-02f,-1.053201593e-02f,3.259184584e-02f,-2.717771381e-02f,4.951885715e-02f,-7.808897179e-03f,1.112366021e-01f,4.898853600e-02f,5.802009720e-03f,-4.439452663e-02f,3.943802789e-02f,1.627320051e-01f,-1.907103136e-02f,7.425275166e-03f,-5.873631686e-03f,-5.231881142e-02f,0.000000000e+00f,-3.952460364e-02f,-4.093258083e-02f,1.695960201e-02f,-3.245169669e-02f,-6.546426099e-03f,-2.441699430e-02f,1.522072870e-02f,-3.919162974e-02f,-3.153195232e-02f,5.999970064e-02f,2.876359224e-02f,3.212521598e-02f,-3.833030909e-02f,3.010106273e-03f,-5.147545412e-02f,1.581601240e-02f,-7.572149625e-04f,7.748054923e-04f,5.537818186e-03f,7.526947558e-02f,7.748054923e-04f,-2.984545752e-02f,-4.076569155e-02f,9.677755646e-03f,-3.905796260e-02f,3.004592098e-02f,8.228069544e-02f,-1.268760674e-02f,6.719870865e-02f,1.170350052e-02f,1.833911426e-02f,-5.146109313e-02f,-4.401578382e-02f,1.694165543e-02f,-1.899266266e-03f,-3.550734371e-02f,1.298627909e-02f,6.232823059e-02f,4.796161503e-02f,-7.652125508e-02f,-1.411524266e-01f,-3.603857011e-02f,1.030647382e-01f,1.639677328e-03f,9.183136746e-03f,2.556440793e-02f,7.096904516e-02f,5.756403878e-02f,1.695960201e-02f,0.000000000e+00f,0.000000000e+00f,-4.560019821e-03f,4.244145472e-03f,-4.660946131e-02f,6.635805219e-02f,6.432968657e-03f,2.078960836e-02f,2.650790662e-02f,7.336475886e-03f,-3.325285390e-02f,-1.150151938e-01f,-1.352148652e-01f,1.938795298e-02f,-5.640701950e-02f,-9.304952621e-02f,-5.773130804e-03f,1.378521323e-03f,-3.026295640e-02f,-5.092161894e-02f,0.000000000e+00f,2.707517706e-02f,0.000000000e+00f,-2.453188039e-02f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,8.498188108e-03f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,-8.678979240e-03f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,0.000000000e+00f,1.522169262e-02f,-6.004998460e-03f,4.937548656e-03f,3.862683848e-02f,-2.580736578e-02f,1.802593237e-03f,-4.918057472e-02f,2.808539663e-03f,-3.425218165e-04f,-1.975610619e-03f,4.422460217e-03f,6.874830462e-03f,5.996841006e-03f,1.257412368e-03f,-4.898853600e-02f,-1.972849667e-02f,1.532761753e-02f};
Input x;float v=0;
for(int j=0;j<inputWidth;++j){x[j]=clamp(z[j]*inv[j],-8.f,8.f);v+=w[j]*x[j];}
static constexpr float h[][inputWidth]={{-2.594442852e-02f,-6.405322999e-02f,4.175790027e-02f,-3.649377823e-02f,2.929194458e-02f,2.309933305e-02f,-3.393963352e-02f,4.259701818e-02f,-9.085615166e-03f,1.287869737e-02f,5.325373635e-02f,-6.228910759e-02f,-3.440711275e-02f,1.390954759e-02f,4.091402143e-02f,3.183189034e-02f,6.140958890e-02f,2.339423262e-02f,-6.592491176e-03f,-8.273654431e-02f,-2.032665908e-02f,-1.826447621e-02f,3.758182749e-02f,-2.436845191e-02f,-2.676912025e-02f,-5.629162490e-02f,-7.462985069e-02f,5.392527580e-02f,4.918498918e-02f,4.789644480e-02f,-5.638996884e-02f,-7.395640016e-02f,-2.786795236e-02f,2.271602489e-02f,5.464502051e-02f,-1.899286546e-02f,-2.172028832e-02f,-3.002452105e-02f,-4.584202915e-02f,1.291280612e-02f,-7.214718312e-02f,1.227992550e-01f,-1.198641118e-02f,3.371626604e-03f,-5.423345417e-02f,4.425432906e-02f,8.868057281e-02f,2.636896446e-02f,-2.031291462e-02f,-5.229534954e-02f,1.043307260e-01f,6.953845173e-02f,-4.626410455e-02f,1.118527353e-02f,-3.308623331e-03f,-3.096069023e-02f,3.827361763e-02f,-7.115959376e-02f,-3.341301158e-02f,-1.313046459e-02f,-2.491016313e-02f,5.211236700e-02f,1.367205288e-02f,7.260002196e-02f,-6.277438253e-02f,2.470063511e-03f,-2.029809356e-02f,5.056053400e-02f,1.034634933e-01f,-1.747910050e-03f,-3.520916030e-02f,1.237312853e-01f,-4.702578485e-02f,6.342719495e-02f,2.403292106e-03f,3.964505717e-02f,2.873030491e-02f,4.493154585e-02f,7.159440219e-02f,5.107209831e-02f,-4.968176037e-02f,8.153891563e-02f,-5.656289309e-02f,-6.776209921e-02f,-2.751329169e-02f,3.701920435e-02f,1.648703706e-03f,-2.424700558e-02f,-4.174851254e-02f,2.739391662e-02f,1.929843426e-02f,3.133434057e-02f,-1.703118347e-02f,5.405005068e-02f,6.759969890e-02f,6.949850917e-02f,2.401377633e-02f,1.084255055e-02f,8.406531811e-02f,-1.005188562e-02f,-3.513626009e-02f,-4.556661472e-02f,-5.946476012e-04f,2.422983386e-02f,1.201309189e-01f,4.590800032e-02f,-3.790655732e-02f,-5.590002984e-02f,-7.335592806e-02f,2.992084948e-03f,-5.754324794e-02f,6.304851919e-02f,3.052864596e-02f,-4.838407040e-02f,-3.247385845e-02f,6.676265597e-02f,-3.126529232e-02f,4.974639043e-02f,-4.177396372e-02f,-3.647296503e-02f,-5.911196396e-02f,-6.459131837e-04f,1.456917357e-02f,-1.189879328e-01f,-1.906341501e-02f,2.737538517e-02f,-4.883000255e-02f,5.370615423e-02f,2.296250314e-02f,-4.968084022e-02f,-5.367664620e-03f,-5.136433616e-02f,-5.365773663e-02f,-2.660924196e-02f,-1.270185262e-01f,-1.812265068e-02f,3.541281447e-02f,-6.934505422e-03f,-9.226242453e-02f,4.250774160e-02f,-6.676863879e-02f,3.903689608e-02f,5.960921198e-02f,2.869406017e-03f,1.262068190e-02f,2.516537532e-02f,1.928887703e-02f,-7.489759475e-03f,2.902986668e-02f,7.473884523e-02f,-2.937452681e-02f,2.944048867e-02f,-4.318610951e-02f,7.340277731e-02f,4.798591882e-02f,-1.070467010e-01f,-1.224395260e-02f,-1.029837951e-01f,-3.502219915e-03f,-3.931343928e-02f,2.253931575e-02f,-1.094322652e-01f,-2.423279174e-02f,-1.678192988e-02f,-4.286224395e-02f,-6.292876601e-02f},{-1.116159484e-01f,7.237415761e-02f,-4.561846331e-02f,4.395524412e-02f,4.367308691e-02f,-5.393061414e-02f,-8.157506585e-02f,9.500562213e-03f,-6.595730782e-02f,-8.920577168e-02f,-3.501796722e-02f,-4.602897540e-02f,1.629639603e-02f,-9.740626439e-03f,-2.827407606e-02f,-1.215253696e-01f,-5.380452424e-02f,1.610310934e-02f,1.019119099e-02f,5.651985109e-02f,5.901588127e-02f,3.173294663e-02f,-5.731464922e-02f,6.227034330e-02f,2.902758867e-02f,8.513113484e-03f,4.271237180e-02f,4.116140679e-02f,-6.265805662e-02f,8.471094072e-02f,4.240906984e-02f,5.734209716e-02f,5.769085139e-03f,-3.188385814e-02f,-1.151176468e-01f,-6.731151789e-02f,2.417596057e-02f,-7.907035202e-02f,2.064816281e-02f,-6.140786782e-02f,-3.752420098e-02f,9.289741516e-02f,-9.210270643e-02f,1.694641542e-03f,-5.036036670e-02f,-8.703015745e-02f,5.207169801e-02f,5.839356780e-02f,-6.221951544e-02f,3.431465849e-02f,-2.219420858e-02f,4.980896786e-02f,-8.133779280e-03f,-1.817505807e-02f,-2.725288458e-02f,9.320946783e-02f,-2.591328509e-02f,7.417852432e-02f,2.065763110e-03f,-4.813520610e-02f,-3.638973832e-02f,-2.313356474e-02f,-3.692440316e-02f,4.433432966e-02f,-8.703733981e-02f,-4.068072513e-02f,1.564359106e-02f,-1.342317611e-01f,-3.265227750e-02f,-4.240156710e-02f,-4.196850210e-02f,-5.001560971e-02f,-2.161598532e-03f,-1.524705626e-02f,6.844769418e-02f,-3.662327304e-02f,2.837928385e-02f,2.584816329e-02f,9.428396821e-02f,6.127773598e-02f,-3.553467616e-02f,-2.956669591e-02f,-3.824477643e-02f,2.601349726e-02f,-9.324025363e-02f,-7.292606402e-03f,-2.874086052e-02f,-4.709007684e-03f,-7.353246212e-02f,3.346863016e-02f,5.659565330e-02f,-2.091142535e-02f,-1.376676653e-02f,-1.348809688e-03f,1.067596301e-01f,1.181079522e-01f,-2.457734570e-02f,9.449973702e-02f,-5.494448170e-02f,-3.601321951e-02f,-7.771772891e-02f,1.877017878e-02f,-1.451209933e-01f,8.510066196e-03f,2.402885444e-02f,-3.117660526e-03f,9.995505959e-02f,-2.182522975e-02f,5.405838415e-02f,7.861904800e-02f,-5.448870733e-02f,3.277305141e-02f,-1.153959613e-02f,-3.160277382e-02f,7.099910080e-02f,-7.054120302e-02f,2.827912197e-02f,4.376455396e-02f,1.659688354e-02f,-1.274428368e-01f,3.537395597e-02f,3.152244166e-02f,5.404513329e-02f,-8.825602382e-02f,-7.941753604e-03f,4.118427634e-03f,-1.821788773e-02f,-4.683180712e-03f,4.607877880e-02f,3.259793296e-02f,-1.632798463e-02f,2.085887641e-02f,1.161995158e-02f,2.328692563e-02f,-6.205005944e-02f,-5.690016225e-02f,4.974216688e-03f,-4.470057786e-02f,-2.294163406e-02f,-4.437492788e-02f,-6.361830235e-02f,7.479397207e-02f,5.735842884e-02f,-6.574846804e-02f,-2.870679274e-02f,-7.258514315e-02f,-5.488584191e-02f,-1.922502667e-01f,-2.207050472e-01f,2.529920265e-02f,-2.469633333e-02f,-1.000718325e-01f,6.236853078e-02f,-6.334034353e-02f,1.989945304e-03f,4.346232861e-02f,2.157889307e-02f,1.058569271e-02f,4.216681793e-02f,1.433406956e-02f,9.049504995e-02f,4.395185411e-02f,5.696390569e-02f,-6.213201955e-02f,5.320204655e-04f,7.492097467e-02f},{-7.982756943e-02f,7.440831512e-02f,1.059877053e-01f,-3.832162172e-02f,-3.092690930e-02f,6.199992448e-02f,-6.279555708e-02f,-7.104610652e-02f,-6.440044940e-02f,2.421569824e-02f,-4.264438525e-02f,7.602471858e-02f,-2.620016038e-02f,-5.185565725e-02f,4.076676816e-02f,-5.337319523e-02f,7.769352198e-02f,-1.184168924e-02f,7.793137804e-03f,-5.709383637e-02f,-4.100026935e-02f,-4.466406256e-02f,-8.612953126e-03f,1.083764061e-02f,7.412332576e-03f,-8.083200455e-02f,-6.273748726e-02f,-2.211607248e-02f,5.041330308e-02f,2.328253910e-02f,-6.027482077e-02f,-4.346188158e-02f,-2.832062356e-02f,1.061313450e-01f,4.642011598e-03f,8.787681162e-02f,-4.568582028e-02f,-9.470182657e-02f,-7.155311108e-02f,2.246718481e-02f,1.233362406e-02f,-6.112021580e-02f,-1.848584041e-02f,3.971209377e-02f,4.879522603e-03f,-4.117225297e-03f,-6.598702073e-02f,7.557266951e-02f,-8.069048077e-02f,1.916129258e-03f,3.992376477e-02f,-3.794390336e-02f,1.555628981e-02f,6.481828541e-02f,3.233407810e-02f,7.080398500e-03f,7.270535082e-02f,4.068905953e-03f,9.647351503e-02f,3.656582162e-02f,-2.953630500e-02f,5.333325639e-02f,1.235047542e-02f,-5.709360167e-02f,-7.200585678e-03f,-4.693481326e-02f,8.834609389e-02f,-9.287862480e-02f,2.364169061e-02f,-7.896572060e-05f,-5.297088251e-02f,1.545117889e-02f,-8.464183658e-02f,-6.808304787e-02f,-6.172957271e-02f,-6.813079119e-02f,1.780896448e-02f,2.061953768e-02f,-2.635500021e-02f,7.844342291e-02f,5.178615451e-02f,-3.516980261e-02f,-1.920275763e-02f,1.175714508e-01f,5.706261471e-02f,-5.748765171e-02f,-1.480137114e-03f,3.389091045e-02f,-6.709457189e-02f,3.575554863e-02f,-4.922309518e-02f,4.614985362e-02f,1.008707285e-02f,6.627400964e-02f,1.028478891e-01f,7.928108424e-02f,-6.540231407e-02f,-2.991938265e-03f,-2.822299674e-02f,6.304313987e-02f,3.681731224e-02f,-5.947156250e-02f,-1.033262312e-01f,3.029837273e-02f,5.831369199e-03f,-6.006047502e-02f,-1.076998338e-01f,4.014392942e-02f,-3.050814383e-02f,6.983953714e-02f,-8.163613081e-02f,-4.340153560e-02f,-7.023636252e-02f,-8.662223071e-02f,4.500495270e-02f,1.784561947e-02f,-3.364572674e-02f,2.579886094e-02f,6.869162619e-02f,-6.290542334e-02f,5.405025557e-02f,4.903291166e-02f,-5.333882570e-02f,-1.592942625e-01f,1.819943078e-02f,3.553786501e-02f,6.104290113e-02f,1.773831062e-02f,8.161243051e-02f,1.729023829e-02f,6.973336637e-02f,6.204191968e-02f,-1.084169652e-02f,4.364571348e-02f,-5.267580971e-02f,3.098430298e-02f,-3.141777590e-02f,-6.114321575e-02f,-4.287790135e-02f,-6.036084145e-03f,-5.327946693e-02f,-5.015912279e-02f,1.855058037e-02f,5.229454115e-02f,6.455438584e-02f,-6.574956328e-02f,-1.888555661e-02f,-1.401850209e-02f,-7.440519333e-02f,6.864983588e-02f,7.123515010e-02f,6.815827638e-02f,7.687579840e-02f,-7.800997794e-02f,6.065064669e-02f,-5.870771129e-03f,-3.467453271e-02f,4.048668221e-02f,2.615089528e-02f,-6.214091927e-02f,-2.146428823e-02f,-4.913508520e-02f,1.112313196e-02f,-6.935322285e-02f,5.592235550e-02f,-6.194047630e-02f},{-3.888816759e-02f,2.158788964e-02f,4.136879742e-02f,5.538609996e-02f,-3.817554936e-02f,4.650790617e-02f,-4.871966690e-02f,-5.817962438e-02f,-6.201710179e-02f,-8.063627034e-02f,6.839247793e-02f,-1.368077472e-02f,-1.384550035e-01f,3.628021479e-02f,-5.782519281e-02f,-5.597480107e-03f,5.617398769e-03f,4.572732747e-02f,-4.883187823e-03f,-1.446527690e-01f,8.581778407e-02f,-1.486340258e-02f,-9.562399238e-02f,-8.748243749e-02f,-3.020470403e-02f,-6.716601551e-02f,3.036015481e-02f,-9.286356717e-02f,5.414120108e-02f,1.714845002e-02f,9.137320332e-03f,-5.712236837e-02f,-4.386375099e-02f,-1.819224842e-02f,3.197617084e-02f,1.409090310e-02f,-1.224755403e-02f,1.064373553e-01f,-5.488641560e-02f,9.623744339e-02f,-1.564041129e-03f,-6.922908127e-02f,-4.634933546e-02f,-3.538122028e-02f,1.110165939e-01f,-5.158774555e-02f,5.346979480e-03f,-5.388114974e-02f,1.277757883e-01f,3.444522619e-02f,-8.892297745e-03f,-3.306310624e-02f,-4.824295640e-02f,-7.745901495e-02f,-7.854791731e-02f,6.031205878e-02f,-1.218895614e-01f,6.991845369e-02f,7.533400785e-04f,7.631005347e-02f,-1.115111336e-01f,4.553042352e-02f,2.960705198e-02f,-1.080821641e-02f,-1.314904448e-02f,-6.947763264e-02f,3.591189161e-02f,-6.128650904e-02f,-2.220104076e-02f,7.282265462e-03f,-5.046903715e-02f,-6.142586842e-02f,-2.582785441e-03f,5.219567567e-02f,4.118976370e-02f,-6.392324716e-02f,-6.439520977e-03f,5.378323421e-02f,2.736625448e-02f,-5.733759142e-03f,2.283264138e-02f,-1.575322747e-01f,4.004232213e-02f,9.298306704e-02f,-1.257298794e-02f,1.559994277e-02f,9.093832923e-04f,-3.836060315e-02f,7.035467774e-02f,-4.688552022e-02f,6.303970516e-02f,-7.494700700e-02f,-1.178320795e-01f,4.541639984e-02f,-8.389828354e-02f,-4.960599914e-02f,2.978348930e-04f,-1.196401659e-02f,9.184702486e-02f,-1.223987620e-02f,-2.143461071e-02f,-2.212581597e-02f,2.031924576e-02f,-5.340282619e-02f,-5.756826326e-02f,5.166129768e-02f,8.474688046e-03f,-2.351096086e-02f,-2.698617615e-02f,-6.964198314e-03f,1.040116996e-01f,5.654675514e-02f,7.110308856e-02f,1.304483265e-01f,-4.973076656e-02f,1.791434735e-02f,-3.904327378e-02f,1.692997525e-03f,4.637202993e-02f,8.932949603e-02f,1.074525043e-01f,-5.569083616e-02f,7.010653615e-02f,-4.089646786e-02f,-8.844386041e-02f,9.349697828e-02f,5.810701102e-02f,9.970501065e-03f,-6.329772621e-02f,5.069462582e-02f,8.562277257e-02f,1.554427855e-02f,-4.226604849e-02f,-5.030749366e-03f,-3.869979875e-03f,5.291963369e-02f,4.283815250e-02f,-5.093477666e-02f,-9.724446386e-02f,-7.689969987e-02f,-3.172367811e-02f,7.111564279e-02f,3.503188491e-02f,2.415460721e-02f,-3.223169222e-02f,1.878644526e-02f,-1.143216714e-02f,-1.012920812e-01f,-6.465232372e-02f,-1.001937911e-01f,-2.359549887e-02f,9.664477780e-03f,-5.205173977e-03f,-6.302215625e-03f,2.669750340e-02f,-8.975674212e-02f,1.369739231e-02f,-5.055129901e-02f,-6.355535239e-02f,-8.794675022e-02f,-2.647684142e-02f,-8.364950866e-02f,-3.123369068e-02f,8.025611192e-02f,-2.490183152e-02f,-1.862873137e-02f},{-3.668106720e-02f,2.577553503e-02f,3.008167818e-02f,5.800529942e-02f,4.372117296e-02f,-8.966008201e-03f,-8.824925125e-02f,-5.876856297e-02f,-2.176788077e-02f,-2.986524999e-02f,-6.990648061e-02f,-4.995891079e-02f,2.031771280e-02f,1.368877664e-02f,2.013583668e-02f,-5.038954690e-02f,3.650440276e-02f,-3.973896801e-02f,-2.740084380e-02f,2.226156928e-02f,-1.578339934e-02f,2.571446449e-02f,-1.217654254e-02f,-6.179318298e-03f,-7.935385220e-03f,-2.016970143e-02f,-3.652793355e-03f,5.258029327e-02f,-5.518310354e-04f,4.946517199e-02f,6.049768627e-02f,4.241802171e-02f,-7.035557926e-02f,5.293762684e-02f,-1.922915131e-02f,-1.024472341e-02f,4.899661615e-02f,-5.272132531e-02f,-1.483584382e-02f,-2.278249711e-03f,2.471977472e-02f,-7.630956918e-02f,-1.106837466e-01f,-1.369332988e-02f,3.601507097e-02f,2.372257225e-02f,5.557505041e-02f,-6.957814097e-02f,3.408289328e-02f,4.493961856e-02f,5.622122809e-02f,-1.072262898e-01f,-1.116674393e-01f,-4.416733980e-02f,-4.686037451e-02f,7.396806777e-02f,5.013135076e-02f,1.843361184e-03f,5.878676660e-03f,3.145805374e-02f,-4.863668606e-02f,-3.548282385e-02f,-6.106135622e-02f,-1.792287454e-02f,-4.223907739e-02f,1.615039259e-02f,9.426625073e-02f,3.162322193e-02f,-5.575746670e-02f,-8.252191544e-02f,-2.868912369e-02f,3.119785339e-02f,-4.194817692e-02f,4.175640270e-02f,1.649866812e-02f,2.342543378e-02f,-1.569258608e-02f,-3.143521026e-02f,8.371075243e-02f,6.864504423e-03f,-3.629580885e-02f,-9.177229553e-02f,3.896088153e-02f,2.678959072e-02f,-8.408061415e-02f,3.106367402e-02f,3.455002606e-02f,3.571456298e-02f,-6.369876117e-02f,4.063915089e-02f,6.143129617e-02f,3.181326017e-02f,-8.924406767e-02f,3.338782489e-02f,2.361595817e-02f,-1.557523757e-02f,-1.914909668e-02f,-1.505488716e-02f,-1.172241103e-02f,7.145716995e-02f,1.667197049e-02f,3.920218349e-02f,-3.705478832e-02f,-3.386858478e-02f,-1.015443131e-01f,-3.500151634e-02f,3.652647883e-02f,-3.167395666e-02f,3.760015219e-02f,-9.822575003e-02f,-2.421208471e-02f,1.393691357e-02f,1.100855134e-02f,-1.358196605e-02f,1.687925868e-02f,1.697935723e-02f,2.242468297e-02f,-1.581928506e-02f,-8.146815747e-02f,-3.361729160e-02f,8.067945391e-02f,7.371094078e-02f,8.633140475e-02f,5.584343895e-02f,-5.611147732e-02f,8.994568139e-02f,9.188769013e-02f,-1.724046282e-02f,4.350716621e-02f,8.612072095e-03f,-4.609702900e-02f,-5.960081890e-02f,-9.001164138e-02f,-5.944864452e-02f,-7.222989202e-02f,-6.199826673e-02f,1.138737798e-02f,5.014821514e-02f,4.233634099e-02f,2.012086473e-02f,4.347069934e-02f,1.989243552e-02f,-7.442897558e-02f,-5.784999579e-02f,6.627444923e-02f,2.600207739e-02f,-7.019402087e-02f,-1.827177964e-02f,5.645925645e-03f,3.773829713e-02f,-5.039346591e-02f,-1.382390317e-02f,9.313956834e-03f,-5.292706564e-02f,9.330078959e-02f,-4.200779647e-02f,1.992913894e-03f,-3.879383206e-02f,-8.443751186e-02f,-3.948096558e-02f,4.207211547e-03f,-1.160338707e-02f,-6.506804377e-02f,1.118403301e-02f,4.112680629e-02f,-9.351737052e-02f},{6.394237280e-02f,3.913690150e-02f,2.429527976e-02f,2.143718489e-02f,4.633788019e-02f,-8.799111843e-02f,-2.409251407e-02f,-3.940516151e-03f,-5.964702368e-02f,-1.225650162e-01f,-8.464776725e-02f,-9.034975618e-02f,-4.373324290e-02f,-6.063010916e-02f,5.869944021e-02f,-7.358365506e-02f,-3.860060126e-02f,3.078513034e-02f,5.183906108e-02f,-3.394909576e-02f,4.523900151e-02f,7.309052162e-03f,4.524746165e-02f,-8.064775914e-02f,-2.420532890e-02f,5.938763265e-03f,7.920302451e-02f,-7.739728689e-02f,-5.102610961e-02f,4.494449869e-02f,-4.013226554e-02f,3.446858004e-02f,-9.333154559e-02f,-7.373628765e-02f,1.622304693e-02f,7.520748675e-02f,-2.156214416e-02f,1.765104756e-02f,-3.081709146e-02f,-8.157337457e-02f,4.469601065e-02f,-5.621054769e-02f,-5.301828030e-03f,-1.131131873e-01f,-5.817318335e-02f,2.975274436e-02f,1.398009062e-02f,3.468836099e-02f,-6.419248134e-02f,-5.280420184e-03f,3.090492636e-02f,-1.086424366e-01f,-5.783616751e-02f,3.085910901e-02f,8.168132603e-02f,-6.819018163e-03f,-9.458112717e-02f,4.438640550e-02f,-4.014688358e-02f,-3.449007124e-02f,-8.955150843e-03f,-9.520245716e-03f,-6.192908064e-02f,4.893527180e-02f,-3.133320808e-02f,3.707983298e-03f,-5.009327456e-02f,-8.606775105e-02f,2.952784859e-02f,-2.689170837e-02f,9.304793552e-03f,-5.918894708e-02f,-6.664255261e-02f,1.423861086e-02f,-4.705619812e-02f,6.690728664e-02f,2.088284679e-02f,-1.314236317e-02f,2.439485863e-02f,4.608920962e-02f,-6.222067028e-02f,-4.571925476e-02f,-5.716936290e-02f,8.089299314e-03f,-3.944630921e-02f,-2.519210242e-02f,4.720249027e-02f,-4.474777356e-02f,-4.992287233e-02f,4.904191196e-02f,-4.520127550e-02f,-7.294884324e-02f,5.282546580e-02f,5.233572051e-02f,-1.895366609e-02f,-7.224142551e-02f,5.544050038e-02f,-4.435911775e-02f,-7.545575500e-02f,2.183883078e-02f,-4.292472824e-02f,1.100348867e-02f,1.940610260e-02f,2.321583778e-02f,4.321139678e-02f,-7.460920513e-02f,-2.802043222e-02f,-5.111051351e-02f,8.544112742e-02f,-8.986723423e-02f,-3.233173117e-02f,-1.626412198e-02f,-4.359447211e-02f,2.030823380e-03f,2.272264659e-02f,-8.948501945e-02f,5.165605620e-02f,5.980931222e-02f,7.962684333e-02f,4.417244717e-02f,2.735547535e-02f,7.834816724e-02f,8.345690370e-02f,1.413626317e-02f,-4.847602546e-02f,6.670621037e-02f,9.052156657e-02f,-8.015006781e-02f,5.447080731e-02f,7.535980642e-02f,-3.232018650e-02f,6.679792702e-02f,-4.325847141e-03f,-1.156577561e-02f,-7.102941722e-02f,5.048554018e-02f,-5.553593114e-02f,-6.936527044e-02f,3.984251246e-02f,-3.281163052e-02f,6.369344145e-02f,-9.162778035e-03f,-8.345264196e-02f,-2.459752001e-02f,-1.300815400e-02f,6.110557821e-03f,2.613520995e-02f,8.992389776e-03f,-6.902885437e-02f,3.197138011e-02f,-4.688129574e-02f,-6.493137777e-02f,2.601971477e-02f,-4.791889712e-02f,-2.948925085e-02f,2.411205508e-02f,-5.770261958e-02f,-6.264384836e-02f,-4.035334662e-02f,-1.428413577e-02f,-1.009134650e-01f,-1.108152568e-01f,-3.174197301e-02f,2.889721654e-02f,-3.724296764e-02f,-6.767728925e-02f},{-1.656120643e-02f,2.906370535e-02f,-7.533410937e-02f,4.643340595e-03f,-4.031187668e-02f,2.850987762e-02f,-6.275044288e-03f,2.637466602e-02f,7.267376781e-02f,3.609748930e-02f,3.071987815e-02f,1.468716003e-02f,1.294492185e-01f,2.227811143e-02f,7.082905620e-03f,5.469852313e-02f,5.101766065e-02f,3.077205829e-02f,2.247504331e-02f,1.017757505e-01f,-5.915786326e-02f,1.800526120e-02f,-1.357125398e-02f,9.956175927e-04f,1.290625427e-02f,5.463349074e-02f,6.400948763e-02f,8.396389335e-02f,1.539971307e-02f,6.611163169e-02f,8.856701897e-04f,-3.815075755e-02f,-1.694465056e-02f,-5.999040231e-02f,5.106768012e-02f,1.119642034e-01f,3.342271224e-02f,6.500929594e-02f,8.270890266e-02f,5.042019859e-02f,4.824332893e-02f,-1.582582854e-02f,-2.967635356e-02f,3.834756091e-02f,4.952172190e-02f,-6.390309427e-03f,-5.172913149e-02f,-3.387870640e-02f,1.269801706e-02f,8.757606149e-02f,5.083794822e-04f,9.711792320e-02f,2.597645111e-02f,1.427082121e-01f,7.692100853e-02f,8.553930372e-02f,4.941227660e-02f,-3.003029712e-02f,-4.169826955e-02f,7.606011629e-02f,-1.536624879e-02f,2.282875590e-02f,2.554780617e-02f,-1.075219270e-02f,-5.002990365e-02f,2.472997457e-02f,-8.998314291e-02f,7.712028176e-02f,7.245919853e-02f,-1.894015120e-03f,5.481994245e-03f,1.017155722e-01f,-1.058207732e-02f,-7.858700305e-02f,2.471543476e-02f,5.532174837e-03f,5.935663823e-03f,6.750083528e-03f,3.374396265e-02f,-1.156968717e-02f,3.408661485e-02f,-5.576932430e-02f,-4.912335426e-02f,-9.304336854e-04f,4.594373517e-03f,3.256856464e-03f,4.290161282e-02f,2.930611186e-02f,6.984412670e-02f,-5.912011489e-03f,3.318398818e-02f,-5.704670772e-02f,-8.281291462e-03f,-7.958433777e-02f,1.185393427e-02f,9.511285275e-02f,-9.739820845e-03f,-2.739496343e-02f,4.462533444e-02f,-5.998119712e-02f,3.977672011e-02f,-6.785877049e-02f,-1.042074934e-01f,6.692014635e-02f,1.197261363e-01f,9.368506819e-02f,-4.356643930e-02f,7.976205647e-02f,-7.392953616e-03f,-1.151591912e-02f,-9.880824015e-03f,-3.910592943e-02f,-2.541867457e-02f,-1.075251698e-01f,6.138733029e-02f,-1.188020967e-02f,8.471541107e-03f,1.657826826e-02f,6.440860033e-02f,1.235503145e-02f,5.667121708e-02f,-5.769929290e-02f,3.592308983e-02f,-7.741056383e-02f,8.730071038e-02f,-7.414290309e-02f,1.448741462e-02f,1.154716033e-02f,1.043846644e-02f,-6.086949632e-02f,-2.756473236e-02f,1.674420014e-02f,1.948676258e-02f,-1.395958290e-02f,-3.570969403e-02f,-4.379414022e-02f,5.268242583e-02f,2.067681588e-02f,-6.288393866e-03f,-5.119718984e-02f,-3.083907440e-02f,4.309481382e-02f,-1.077635810e-01f,-1.920277718e-03f,4.707325622e-02f,-3.187691793e-02f,-6.201466173e-02f,-8.312849700e-02f,3.935515881e-02f,4.550318792e-02f,9.042764083e-03f,-2.682414837e-02f,2.283608727e-02f,-6.194477528e-02f,9.146386757e-03f,-3.342434764e-02f,-1.315542758e-01f,-2.585786209e-02f,-5.790353194e-02f,-1.342893392e-01f,-8.134914190e-02f,-1.057614572e-02f,-5.194024369e-02f,-1.421926022e-01f,-7.289557904e-02f,4.348000512e-02f},{-4.821480811e-02f,-7.613471150e-02f,-3.714690730e-02f,6.602219492e-02f,-3.364726901e-03f,-4.045971204e-03f,6.947693229e-02f,5.222436041e-02f,-2.058430202e-02f,-4.343685880e-02f,6.092450395e-02f,1.004247516e-01f,7.505709678e-02f,4.637034982e-02f,1.883365773e-02f,-3.653531149e-02f,-6.423161179e-02f,-2.879065229e-03f,-8.173871902e-04f,6.834718864e-03f,3.303350508e-02f,6.698229909e-02f,2.479080670e-02f,5.762919039e-02f,-1.567162760e-02f,3.154307976e-02f,6.654426455e-02f,5.965392292e-02f,-5.482786521e-02f,5.953287333e-02f,-8.135090023e-02f,-3.760068445e-03f,5.796337873e-02f,-1.152350567e-03f,3.871289641e-02f,-4.697573185e-02f,-2.004053630e-02f,4.613814130e-02f,-4.393169284e-02f,4.098046944e-02f,-4.315126687e-02f,4.666573554e-02f,-1.386806648e-02f,-3.089825809e-02f,8.865841664e-03f,-6.637438387e-02f,-4.407108203e-02f,-5.827818066e-02f,-7.168673724e-02f,6.325577945e-02f,1.038083248e-02f,3.551019356e-02f,4.396599531e-02f,4.266256001e-03f,-4.234002531e-02f,5.053037778e-02f,-9.885425679e-03f,4.680722021e-03f,-7.031005621e-02f,3.087321110e-02f,-6.130664423e-02f,1.127017941e-02f,-3.779314086e-02f,4.603143781e-02f,4.578037187e-02f,1.325687626e-03f,-9.622563608e-03f,-5.262021348e-02f,-1.851261780e-02f,1.213078201e-02f,-8.995643817e-03f,8.234215528e-02f,3.444174305e-02f,2.866733447e-02f,-3.497728705e-02f,-4.658503085e-02f,6.884864718e-02f,4.176595435e-02f,8.253920078e-02f,-4.228996858e-02f,4.895083979e-02f,1.947809197e-02f,3.833907098e-02f,-2.519563586e-02f,-2.747909399e-03f,-9.847198427e-02f,-2.584635839e-02f,8.142718673e-02f,1.120686252e-02f,7.378508151e-02f,-7.369742543e-02f,1.517816447e-02f,-4.214392044e-03f,7.599048316e-02f,-2.069172636e-02f,5.406489223e-02f,-8.487359434e-02f,2.534844680e-03f,6.235079095e-02f,-9.333413094e-02f,-5.486379564e-02f,-6.659912877e-03f,3.479881585e-02f,1.622091862e-03f,-3.986023366e-02f,1.668874919e-02f,-1.797656156e-02f,-7.451924682e-02f,4.182529077e-02f,7.886870205e-02f,-4.558023438e-02f,-4.967827350e-02f,-2.241291292e-02f,-1.784444414e-02f,-4.429239780e-02f,-5.203454942e-02f,4.338544607e-02f,-6.581675261e-02f,-2.350777388e-02f,-3.554842621e-02f,-6.223082915e-02f,5.586540699e-02f,-6.130479649e-02f,-9.317047149e-02f,-5.206333939e-03f,-7.665766776e-02f,-1.246104203e-02f,-2.033509873e-02f,6.017712131e-02f,-6.417675316e-02f,6.212816387e-02f,-6.557943672e-02f,2.000962943e-02f,5.348835513e-02f,3.030751366e-03f,9.850422852e-03f,7.234165072e-02f,5.286792293e-02f,-2.913525328e-03f,7.086578012e-02f,5.663947389e-02f,4.134221375e-02f,-4.027773067e-02f,7.468193024e-02f,5.654368550e-02f,-6.250577420e-02f,-6.514895707e-02f,3.298190609e-02f,-8.984792978e-02f,3.750534356e-02f,3.058766946e-02f,1.204304118e-02f,-3.045783192e-02f,-2.031382732e-02f,-7.146202028e-02f,-3.853152692e-02f,-1.660261676e-02f,-1.165715903e-01f,2.409452107e-03f,-8.030337840e-02f,-8.695677668e-02f,8.042187430e-03f,-4.051501164e-04f,1.848919317e-02f,1.610553265e-02f,5.077006295e-02f},{-1.032127067e-01f,-1.812387630e-02f,8.945705742e-02f,5.520359799e-02f,-1.450681128e-02f,1.262894832e-02f,8.779012412e-02f,5.027031153e-02f,1.703835279e-02f,-5.830520019e-02f,-2.588909678e-02f,-3.656887263e-02f,-1.059886627e-02f,4.027293064e-03f,4.051395878e-02f,3.116674488e-03f,4.226024821e-02f,-1.616525836e-02f,1.002719626e-01f,-3.461838141e-02f,7.117910683e-02f,3.493729979e-02f,-7.093578577e-03f,-3.659756854e-02f,-3.413360193e-02f,7.807620615e-02f,2.446005866e-02f,-1.959079877e-02f,-3.583471850e-02f,-6.975562871e-02f,4.767666757e-02f,1.144806016e-02f,-3.596938774e-02f,8.373605460e-02f,4.634055495e-02f,-2.994816564e-02f,2.674708143e-02f,3.457274660e-02f,-2.690370195e-03f,2.992723882e-02f,2.907741815e-02f,7.177358121e-02f,-3.796450794e-02f,8.878535032e-02f,7.469264418e-02f,9.244611114e-02f,1.845656545e-03f,4.609921202e-02f,1.012445427e-02f,7.031705976e-02f,-1.537455898e-02f,6.575758010e-02f,3.784222156e-02f,-1.482244674e-02f,-5.808628350e-02f,4.825919867e-02f,3.101628833e-02f,1.539501641e-02f,-2.932224050e-02f,4.227492958e-02f,-2.275365964e-02f,-3.972704709e-02f,-2.464171872e-02f,3.805326298e-02f,2.896595560e-02f,-4.491933808e-02f,2.526129037e-02f,9.107951075e-02f,4.877370968e-02f,1.522280648e-02f,-7.129798084e-02f,6.630587578e-02f,5.928210542e-02f,4.090750217e-02f,-4.451053962e-02f,1.270604879e-02f,4.741664976e-02f,-4.481070861e-02f,3.971569613e-02f,-5.037471652e-03f,-3.892230336e-03f,3.751805052e-03f,3.932521865e-02f,7.059101760e-02f,1.587546431e-03f,9.012658894e-02f,3.184083477e-02f,3.836378828e-02f,2.660430223e-02f,4.623451829e-02f,-4.491838161e-03f,1.770221815e-02f,9.582028538e-02f,2.476007119e-02f,3.805247322e-02f,-8.603005111e-02f,-9.693259746e-02f,2.681465261e-02f,-1.045988649e-01f,4.634817690e-02f,-2.673857287e-02f,-1.592589542e-02f,-4.473993089e-03f,4.938850924e-02f,-2.633329481e-02f,2.173130214e-02f,-8.277797140e-03f,5.386625975e-02f,-8.724811673e-02f,-6.127548218e-02f,-1.677559502e-02f,3.922764957e-02f,5.089583620e-02f,-8.682744205e-02f,7.087048888e-02f,-2.353727259e-02f,1.851979271e-02f,-6.132911891e-02f,8.282106370e-03f,-8.888106793e-03f,-4.560958222e-02f,-4.755822942e-02f,-1.837312803e-02f,-1.057564989e-01f,-1.153156999e-02f,-4.070984200e-02f,-6.235687807e-02f,6.790494174e-02f,7.098176330e-02f,-3.132374585e-02f,-6.014743075e-02f,3.456398472e-02f,-1.083178967e-01f,-2.874997444e-02f,-3.665113077e-02f,3.995825723e-02f,4.359161481e-02f,-2.682979591e-02f,8.900597692e-02f,-1.806297153e-02f,-2.747704275e-02f,2.830614895e-02f,-1.173839867e-01f,7.404451072e-02f,1.321030664e-03f,-7.158591598e-02f,-1.655212417e-02f,8.468499035e-02f,1.180373505e-02f,1.075142063e-02f,3.180539981e-02f,-3.377559409e-02f,4.362080619e-02f,-8.284160495e-02f,5.350128934e-02f,-1.029780582e-01f,4.053669982e-03f,-1.250472851e-03f,2.965931408e-02f,-8.973474801e-02f,-8.275076002e-02f,-7.474745065e-02f,-1.622796990e-02f,-4.754688591e-02f,5.756833404e-02f,-2.766069770e-02f},{-5.890402198e-02f,-4.999436811e-02f,-4.265577719e-02f,6.878767163e-02f,7.430408150e-03f,4.463187233e-02f,-3.647309914e-02f,-4.329682514e-02f,6.536271423e-02f,8.828847855e-02f,2.043768764e-02f,4.137212038e-02f,1.453834176e-01f,-6.855172664e-02f,7.766932715e-03f,1.234567165e-02f,-7.055091113e-02f,-3.340883180e-02f,-5.896889046e-02f,7.810698450e-02f,-7.756380830e-03f,6.523100287e-02f,-3.905846924e-02f,-5.091478676e-02f,1.334240753e-02f,-3.690434992e-02f,9.878112376e-02f,-4.869902972e-03f,2.449101023e-02f,-6.611967087e-02f,2.822541073e-02f,5.667763762e-03f,9.591675550e-02f,-2.733677626e-02f,1.235596985e-01f,8.393180370e-02f,-2.060622722e-02f,-3.386206925e-02f,8.702854067e-02f,9.509638883e-03f,9.495876729e-02f,-2.211680822e-02f,7.419870794e-02f,4.783320427e-02f,2.546345070e-02f,-1.324037742e-02f,-6.317837536e-02f,-8.537764661e-03f,1.230397029e-03f,3.347256035e-02f,3.249284998e-02f,1.131755784e-01f,6.953187287e-02f,6.025880203e-02f,-5.555390567e-02f,-2.102692612e-02f,4.032970965e-02f,6.955887191e-03f,5.947505310e-02f,3.148670867e-02f,-1.813662425e-02f,6.549409032e-02f,2.070761658e-02f,4.184051231e-02f,-5.608365312e-02f,-4.078956321e-02f,-4.041768610e-02f,-5.595617369e-02f,-3.681354225e-02f,1.398301683e-03f,-1.603593118e-02f,-2.244649082e-02f,-1.900163293e-02f,-1.492683124e-02f,3.597611189e-02f,-3.637017682e-02f,2.325891145e-02f,8.951447159e-02f,4.966814443e-02f,-4.902721569e-02f,-8.894433267e-03f,7.570455968e-02f,7.361330092e-02f,3.853196651e-02f,-6.627466530e-02f,2.800582163e-02f,2.309265174e-02f,5.964747071e-02f,9.398394078e-02f,2.041031606e-02f,-4.842356965e-02f,9.413942695e-02f,5.850410834e-02f,-4.084355757e-02f,-8.746257983e-03f,6.631191820e-02f,7.769308239e-02f,1.929701678e-02f,4.331782460e-03f,5.833105743e-02f,9.321977943e-02f,-7.603493333e-02f,1.869175956e-02f,3.581742570e-02f,5.062260851e-02f,1.097470298e-01f,-1.663350128e-02f,5.640491471e-02f,-7.145914435e-02f,-5.343892425e-02f,-8.360782266e-02f,-5.455812439e-03f,-6.836005487e-03f,-3.713843971e-02f,-7.218664140e-02f,5.939706787e-02f,8.213558793e-02f,-4.664704576e-02f,-1.487480570e-02f,1.302446797e-02f,-5.330320448e-02f,2.495587617e-02f,3.530640900e-02f,-1.223927438e-01f,-5.445072055e-02f,-1.472736243e-02f,6.875307113e-02f,-9.293321520e-03f,-1.054616878e-04f,1.631079614e-02f,-6.753060222e-02f,5.016022176e-02f,-5.010737479e-02f,-5.003879219e-02f,-4.404994845e-02f,4.404168203e-02f,4.343882948e-02f,6.791756023e-03f,1.824610680e-02f,3.459952772e-02f,1.014196314e-02f,3.256732598e-02f,-1.402706206e-01f,-5.583301559e-02f,6.025270373e-02f,5.253567547e-02f,3.586043790e-02f,-9.638575139e-04f,-3.473416716e-02f,1.091773249e-02f,-9.390499443e-03f,4.253580421e-02f,-4.367718473e-02f,-2.213443071e-02f,-2.085383609e-02f,-8.946653455e-02f,-3.466931358e-02f,-2.480642870e-02f,-7.475128770e-02f,-6.009122357e-03f,-9.161344916e-02f,-8.918705583e-02f,-1.002444997e-01f,-2.614542469e-02f,-1.539884135e-02f,-1.027696300e-02f},{9.029345820e-04f,4.103084654e-02f,2.541412786e-02f,3.699598834e-02f,-1.135666668e-01f,-3.099657223e-02f,-3.887022613e-03f,-8.079349250e-02f,4.505038634e-02f,-2.354977280e-02f,-4.665101692e-02f,-4.273263272e-03f,-3.161011264e-02f,1.490995474e-02f,3.785851179e-03f,-8.437442034e-02f,-7.950728387e-02f,-8.945106715e-02f,-4.003031924e-02f,6.404255331e-02f,-1.199492533e-02f,-8.513144217e-03f,-2.750082687e-02f,-7.216708362e-02f,8.088678005e-04f,2.095825039e-03f,2.282716334e-02f,-1.231241450e-01f,-1.784447208e-02f,4.789061472e-02f,2.878021449e-02f,2.914666943e-02f,-5.805330817e-03f,2.405273542e-02f,-5.886869878e-02f,-6.683599949e-02f,8.402819932e-02f,5.927973986e-02f,6.611226499e-02f,-3.247246146e-03f,-3.805286810e-02f,3.195594996e-02f,-3.336662054e-02f,-9.509803355e-02f,-3.754182160e-02f,-6.482979655e-02f,1.256149914e-02f,-6.887428463e-03f,7.188494503e-02f,-1.116237836e-03f,1.106018201e-01f,2.878580056e-02f,-6.784845144e-02f,-6.278010458e-02f,7.354369015e-02f,4.011775181e-02f,-6.197981536e-03f,3.078329749e-02f,-1.725273952e-02f,6.421735883e-02f,1.073952690e-01f,-7.436218113e-02f,5.807472020e-02f,-4.726978019e-02f,1.875698194e-02f,3.385943547e-02f,-2.685042284e-02f,2.026737854e-02f,7.648277283e-02f,7.839812897e-04f,8.600784838e-02f,4.716443270e-02f,1.805794798e-02f,-7.399697602e-02f,-1.651673019e-02f,-1.570750587e-02f,-6.812051288e-05f,6.794454157e-02f,-5.361406878e-02f,-2.647687495e-02f,5.260632839e-03f,-8.827697486e-02f,-1.383688487e-02f,6.761252880e-02f,2.188977972e-02f,-5.122207478e-02f,1.348199882e-02f,3.766704723e-02f,3.887388110e-02f,6.301039457e-02f,-8.126731962e-02f,1.056212001e-02f,2.089800872e-02f,3.188686818e-02f,-1.047459021e-01f,1.245271601e-02f,-8.046451956e-02f,-2.935716111e-05f,5.652181432e-02f,5.725367460e-03f,-1.065164953e-01f,5.939183012e-02f,-4.844144359e-02f,3.556287289e-02f,-3.571170568e-02f,3.357737558e-03f,-6.897237152e-02f,-3.289145604e-02f,-5.344934762e-02f,-3.521187231e-02f,-9.173556417e-02f,9.899099357e-03f,-3.063564375e-02f,-1.426809374e-02f,1.510632318e-02f,-2.227259800e-02f,-4.269183427e-02f,-1.068407018e-02f,-5.326012149e-02f,2.028355375e-03f,9.504797868e-03f,5.472926423e-02f,6.657997519e-02f,8.012730628e-02f,4.880546127e-03f,-7.217527926e-02f,-4.284313880e-03f,1.618243195e-02f,7.074078918e-02f,8.241613209e-02f,-3.754587844e-02f,5.170520395e-02f,-3.440561891e-02f,3.948748484e-02f,3.334879875e-02f,-3.921798617e-02f,3.715739027e-02f,7.641220093e-02f,4.007867724e-02f,1.144455746e-02f,-3.694973141e-02f,7.984068245e-03f,-5.310799554e-02f,1.541191526e-02f,2.306646295e-02f,-6.730345637e-02f,-6.079201773e-02f,-4.884340987e-02f,-1.339018941e-01f,-4.184739292e-02f,-1.875450090e-02f,4.585646093e-02f,-4.592949152e-02f,5.952065811e-02f,-4.588655662e-03f,-3.059139289e-02f,-1.972650178e-02f,-1.034700871e-01f,-5.195652694e-02f,-4.216768965e-02f,-1.009043530e-01f,-2.741321549e-02f,-2.145975456e-02f,1.994778216e-02f,3.539814614e-03f,-3.203083761e-03f},{-3.599719331e-02f,-7.179928571e-02f,-1.130667049e-02f,5.398015678e-02f,-5.923497677e-02f,4.665929824e-02f,-7.136961818e-02f,-4.429334402e-02f,4.537510127e-02f,-3.689652309e-03f,7.942810655e-02f,-4.508989677e-02f,-6.074286252e-02f,-5.354838073e-02f,3.093123622e-02f,-6.251665950e-02f,-4.965380207e-02f,-6.339476258e-02f,6.150263175e-02f,4.177031294e-02f,7.878579199e-02f,-6.735243648e-02f,-5.664561316e-02f,5.503451824e-02f,-4.307509214e-02f,-8.077015728e-02f,2.970510349e-02f,7.350223139e-03f,3.157803556e-03f,-3.605170175e-02f,-8.694329113e-02f,2.531664819e-02f,5.682590231e-02f,-6.606609374e-02f,-5.917090923e-02f,2.443112433e-02f,1.605509780e-02f,5.379680544e-02f,-6.002389640e-02f,5.711791664e-02f,1.552062854e-02f,1.098350808e-02f,-2.406599931e-02f,2.223371714e-02f,7.631660253e-02f,-3.147781640e-02f,-3.893867880e-02f,-7.186834514e-02f,-8.775044233e-03f,1.760106720e-02f,-1.774521731e-02f,8.936312050e-02f,3.365251049e-02f,4.507128149e-02f,4.081961885e-02f,5.165532604e-02f,5.721078441e-02f,-3.954415768e-02f,-4.821295291e-02f,-4.872473329e-02f,6.076608598e-02f,6.771808118e-02f,1.407196932e-02f,-3.572162986e-02f,3.308179229e-02f,1.016257983e-02f,-2.839615569e-02f,2.610838041e-02f,5.565222725e-02f,-3.579599783e-02f,1.972950436e-02f,7.139027119e-03f,4.523142427e-02f,-5.198020488e-02f,-2.949922159e-02f,-5.418166146e-02f,-1.551066525e-02f,8.092386648e-03f,-1.534506958e-02f,4.262341931e-02f,2.972345799e-02f,-1.423725951e-02f,2.061360329e-02f,4.721471108e-03f,-5.642283708e-02f,-5.949911848e-02f,5.458064750e-02f,2.999523655e-02f,3.740600124e-02f,2.816228010e-02f,7.142601162e-02f,1.832676306e-02f,2.124532871e-02f,7.059478760e-02f,4.044738784e-02f,4.840162024e-02f,-1.458547637e-02f,4.059904814e-02f,5.091432482e-02f,-5.412623286e-03f,2.829699591e-02f,-9.084741026e-02f,-6.230685115e-02f,-8.995463140e-04f,3.562640771e-02f,5.801269040e-02f,-6.782397628e-02f,-8.582527936e-02f,5.815477297e-02f,-4.364234582e-02f,-1.635621674e-02f,-5.513107404e-03f,2.867664397e-02f,1.806007698e-02f,3.628328070e-02f,6.183471531e-02f,3.554487973e-02f,-3.929014504e-02f,4.100316390e-02f,-3.793997318e-02f,-2.851945348e-02f,-7.638087124e-02f,5.759515613e-02f,2.306457609e-02f,-4.777882248e-03f,-6.768079009e-03f,5.307281017e-02f,4.241378605e-02f,4.222107306e-02f,-1.636499353e-02f,-8.339860290e-02f,-7.189834141e-04f,-6.925836951e-02f,4.055416677e-03f,5.648485571e-02f,-6.018980592e-02f,-6.086174399e-02f,6.305946410e-02f,-6.307378411e-02f,1.189953927e-02f,8.035547100e-03f,-9.268644266e-03f,-7.899150252e-02f,-2.022703923e-02f,3.342860192e-02f,-1.320026908e-02f,8.031763136e-03f,-1.356614660e-02f,5.440149084e-02f,-1.492231339e-02f,-4.787987471e-02f,4.894379899e-02f,-5.540613085e-02f,2.658208460e-02f,5.242824182e-02f,4.013675451e-02f,-8.989416063e-02f,-7.539767027e-02f,2.708144113e-02f,-9.746798128e-02f,-4.108615592e-02f,2.254500613e-02f,-4.129854590e-02f,-6.931107491e-02f,-5.987590551e-02f,2.900692821e-02f},{8.982285857e-02f,-7.573081553e-02f,-4.762578011e-02f,4.228792340e-02f,-6.304810196e-02f,5.820601434e-02f,-3.377553821e-02f,6.936588883e-02f,5.722612143e-03f,1.460023317e-02f,3.781770915e-02f,1.500347536e-02f,9.188180417e-02f,8.349180222e-02f,3.324789181e-02f,8.425147831e-02f,-1.580293663e-02f,-9.889340959e-03f,-5.358420685e-02f,-3.689548746e-02f,6.290207058e-02f,-1.437026914e-02f,1.158149075e-02f,-7.483522594e-02f,4.716697335e-02f,6.464333832e-02f,-3.576597571e-02f,4.524086043e-02f,1.130503342e-01f,1.264401376e-01f,-3.311545122e-03f,-4.037923366e-02f,-5.207253993e-02f,-8.777405322e-02f,6.199405715e-02f,9.679563344e-02f,3.465767205e-02f,9.531825781e-02f,6.834498793e-02f,5.228751805e-03f,7.991914451e-02f,6.853179634e-02f,6.003948301e-02f,5.631082505e-02f,-1.860238798e-02f,6.688578427e-02f,-1.058316231e-01f,-5.854306370e-02f,-8.421386778e-02f,-8.118662052e-03f,-1.454614475e-02f,-1.707693678e-03f,1.033970043e-01f,1.284312755e-01f,-5.764277279e-02f,1.187783852e-01f,2.282427996e-02f,2.653698251e-02f,4.048161581e-02f,6.935410947e-02f,-6.905811280e-02f,-8.127270266e-03f,-1.897376776e-02f,8.625168353e-02f,-1.024868246e-02f,4.153855552e-04f,5.683822557e-02f,6.001996621e-02f,3.792656958e-02f,1.104044393e-01f,-3.738251328e-02f,7.606352121e-02f,1.030409262e-01f,1.582435146e-02f,8.474797010e-03f,1.483942289e-02f,3.836188093e-02f,4.321625922e-03f,4.440558329e-02f,3.621687740e-02f,3.438436240e-02f,1.174126193e-01f,-4.870630801e-02f,-6.041822955e-02f,-3.817386925e-02f,-1.822419465e-02f,5.753364041e-02f,-1.128502656e-02f,4.156560823e-02f,-2.992418222e-02f,1.209664196e-01f,4.080575425e-03f,-1.930204034e-02f,1.303440239e-02f,-2.601135336e-02f,1.019446999e-01f,7.771743089e-02f,5.516049638e-02f,1.279117726e-02f,-6.065667607e-03f,5.785183981e-02f,-7.502664626e-02f,-2.875968814e-02f,4.107181728e-02f,4.183725640e-02f,-1.644310541e-02f,-1.346124560e-01f,9.061019868e-03f,5.819501728e-02f,-2.562095411e-03f,-3.828116879e-02f,8.652102202e-03f,5.459518731e-02f,-4.540109262e-02f,3.402713686e-02f,4.783904180e-02f,-3.627295047e-02f,-9.161139023e-04f,1.256012917e-01f,7.631064951e-02f,2.731022239e-02f,-3.497168794e-02f,7.247339934e-02f,3.113500541e-03f,1.963276416e-02f,-3.092873469e-02f,-4.055890441e-02f,3.268073499e-02f,-3.175171465e-02f,-1.774360798e-02f,-2.519959770e-02f,7.335909456e-02f,-9.808801860e-02f,-3.213209333e-03f,1.681853458e-02f,7.182772458e-02f,2.433974668e-02f,2.199183404e-02f,-3.894807771e-02f,-5.238204449e-02f,-6.577247381e-02f,1.571969129e-03f,-1.170899421e-01f,-5.281335115e-02f,-2.861788310e-02f,2.240016684e-02f,1.055635139e-02f,-1.777887903e-02f,-5.344788730e-02f,8.874794841e-02f,7.587537169e-03f,2.136238851e-02f,3.612518311e-02f,-6.858149171e-02f,5.435315520e-02f,-4.089919105e-02f,-5.201010779e-02f,-6.211168319e-02f,-9.223939478e-02f,-1.323191077e-01f,-7.985769212e-02f,-9.070454538e-02f,-7.485377043e-02f,-1.274228394e-01f,-4.690494761e-02f,-4.826603830e-02f},{-3.192699701e-02f,-6.172093377e-02f,1.075485721e-02f,-4.680916667e-02f,-2.229792438e-02f,2.243499458e-02f,-2.494433150e-02f,-8.615083061e-05f,6.336600333e-02f,-8.952638134e-03f,6.469708681e-02f,4.089628905e-02f,-6.165953819e-03f,-1.200155541e-02f,4.390566144e-03f,-3.720138595e-02f,2.577120997e-02f,7.064031810e-02f,-2.196173370e-02f,-7.235153764e-02f,3.735487908e-02f,-7.987795770e-02f,-8.383309841e-02f,-6.697089225e-02f,-3.682224080e-02f,-6.387213618e-02f,2.873143554e-02f,4.823191091e-02f,-3.252898529e-02f,1.909886370e-03f,-2.483602241e-02f,-8.184309304e-02f,6.931522395e-03f,-8.537642658e-03f,5.652231723e-02f,-4.281952605e-02f,3.992186487e-02f,-3.648386896e-02f,-7.153439522e-02f,-5.993439350e-03f,5.011637509e-02f,-4.115218669e-02f,6.170090288e-02f,-2.114992961e-02f,7.994712889e-02f,2.953826822e-02f,5.523579940e-02f,-3.074562363e-02f,6.967705488e-02f,4.486754537e-02f,-1.099740341e-01f,-5.564251915e-02f,-9.156631678e-02f,-5.104245991e-02f,-1.898072101e-02f,-3.360531107e-02f,-5.800424889e-02f,1.782537065e-02f,-2.943896130e-02f,-2.220707200e-02f,-3.582216101e-03f,-7.538768649e-02f,1.760664023e-02f,-3.803464398e-02f,7.634323090e-02f,-3.679789975e-02f,1.726419851e-02f,5.064524338e-02f,-1.412929874e-02f,9.260407835e-02f,-1.809404977e-02f,1.902920939e-02f,2.924404107e-02f,6.874778867e-02f,-7.045529038e-02f,-5.120706931e-02f,-2.782967873e-03f,-3.648015484e-02f,-8.397990465e-02f,8.239198476e-02f,1.109560579e-02f,-8.899168670e-02f,1.118987724e-01f,-1.490251422e-01f,1.233360637e-02f,3.584155440e-02f,4.795894027e-03f,6.370153278e-02f,-5.822699144e-02f,-1.811654866e-02f,9.646932594e-04f,5.737496167e-02f,-5.817905813e-02f,-7.579369098e-02f,-3.989443555e-02f,3.204777464e-02f,5.145194381e-02f,-4.011781141e-02f,8.889596909e-03f,4.702536389e-02f,1.769862697e-02f,-3.073557280e-02f,5.666050129e-03f,2.741105109e-02f,8.223108947e-02f,2.284263261e-02f,7.382418215e-02f,5.992536619e-02f,-6.516778469e-02f,1.699879766e-02f,-4.229440354e-03f,-3.971665725e-02f,-2.820467204e-02f,8.977553807e-03f,-8.291979134e-02f,4.184944555e-02f,-4.670459777e-02f,-6.790009886e-02f,-7.319467515e-02f,7.049024105e-02f,5.917062238e-02f,5.129516497e-02f,8.286206424e-02f,5.053366069e-03f,-6.330716424e-03f,1.109667569e-01f,2.015900612e-02f,2.797300182e-02f,2.548245341e-02f,6.295994669e-02f,-3.555604443e-02f,1.106266864e-02f,2.939119749e-02f,-6.827384233e-02f,-9.366273880e-02f,7.661429048e-02f,-2.301364951e-02f,5.610272661e-02f,-4.727737233e-02f,-2.428196557e-02f,-4.244447127e-02f,2.627701126e-02f,-9.489712864e-02f,-1.511304267e-02f,-4.044025019e-02f,-7.727319375e-03f,-6.515371054e-02f,-1.412771493e-01f,-1.305568963e-01f,6.408258528e-02f,5.842726305e-02f,-1.139609050e-02f,4.429197684e-02f,-3.085732274e-02f,7.419154048e-02f,-8.463264257e-02f,2.505658194e-02f,1.163680200e-02f,-8.867554367e-02f,-9.173710644e-02f,-5.568401888e-02f,-5.098740384e-02f,2.945863456e-02f,-4.486014077e-04f,-9.479063749e-02f,-2.038975246e-02f},{1.450316235e-02f,-2.547978051e-02f,-1.016046852e-01f,1.821283624e-02f,9.759546816e-02f,8.983563632e-02f,-8.668302745e-02f,9.231792390e-02f,-4.974572361e-02f,1.630738936e-02f,-1.832053997e-02f,-5.122610927e-02f,8.821395226e-03f,-4.954658821e-02f,8.676296473e-02f,-3.775885701e-02f,-6.176288426e-02f,-6.457599998e-02f,5.269013718e-02f,-7.666271925e-02f,-1.275005471e-02f,-8.843554556e-02f,-3.174285870e-03f,3.351463005e-02f,-7.324425876e-02f,-8.236166090e-03f,-5.489671975e-02f,-1.426026877e-02f,-5.450829491e-02f,1.704112068e-02f,4.015092924e-02f,6.126189232e-02f,-4.694868252e-02f,-9.878841788e-02f,-2.541768923e-02f,8.628760278e-02f,-4.614152759e-02f,5.950624123e-02f,-3.199184686e-02f,-1.844213530e-02f,2.625788376e-02f,4.397022538e-03f,-2.978966385e-02f,9.876443073e-03f,5.667592213e-02f,1.830076985e-02f,6.744323671e-02f,1.868695207e-02f,-4.219191149e-02f,2.014438249e-02f,-9.542205185e-02f,4.447901249e-02f,4.824932665e-03f,9.849523008e-02f,2.721433155e-02f,-9.777215868e-02f,1.371261477e-02f,4.189551622e-02f,3.013422713e-02f,6.448412314e-03f,-2.546542697e-02f,-2.840868756e-02f,1.056796778e-02f,-2.869979851e-02f,-3.914884292e-03f,-9.504434466e-02f,-6.065514870e-03f,-6.934667937e-03f,3.390970826e-02f,6.525058299e-03f,8.907335810e-03f,-6.790015846e-02f,4.777854308e-03f,2.072533220e-02f,1.335921697e-02f,1.159503963e-02f,-5.644970015e-02f,-4.450988024e-02f,-8.026054502e-02f,-7.200266421e-02f,7.332920283e-02f,-1.202670261e-01f,-3.555893898e-02f,-7.849191874e-02f,3.282329068e-02f,-4.206778854e-02f,6.694002450e-02f,7.025260478e-02f,-1.994455932e-03f,-3.418126330e-02f,2.453765087e-02f,2.742038295e-02f,-1.541087776e-01f,4.295981489e-03f,-5.415651947e-02f,-3.520996496e-02f,5.754191801e-02f,-1.826018095e-02f,1.156494208e-02f,-7.339151204e-02f,5.247712880e-02f,1.087404564e-01f,-8.694382757e-02f,-2.457114635e-03f,6.142201275e-02f,3.191574663e-02f,1.973944716e-02f,-1.060966849e-01f,1.632525027e-02f,-4.869957268e-02f,4.986908659e-02f,5.122814700e-02f,-5.994984135e-02f,-1.175085977e-01f,1.453810488e-03f,1.670718752e-02f,3.148100153e-02f,3.445203602e-02f,5.800519884e-02f,-6.157401949e-02f,-7.505308837e-03f,4.952879623e-02f,-3.084081970e-02f,3.405770659e-02f,-7.489704341e-02f,1.019558385e-01f,-3.714708239e-02f,-2.195401862e-02f,-6.633806974e-03f,-1.243065391e-02f,4.741953313e-02f,-3.051575273e-02f,-2.417930029e-02f,1.998372748e-02f,-7.130628824e-02f,-6.736062467e-02f,-6.514916569e-02f,-5.916975811e-02f,-9.504409879e-02f,2.201641910e-02f,2.867287584e-02f,-1.574537344e-02f,-6.156494096e-02f,7.862298749e-03f,2.037299983e-02f,-6.489206851e-02f,-1.989188418e-02f,-6.981129944e-02f,-1.448038369e-01f,3.232475743e-02f,-2.747409232e-02f,-2.131616697e-02f,1.804916188e-02f,5.299833417e-02f,5.044919252e-02f,-5.936529115e-02f,-1.010800451e-01f,-3.527169302e-02f,7.648212370e-03f,-9.555868059e-02f,-1.185371950e-01f,-4.670011625e-02f,-4.950279742e-02f,-1.187126786e-01f,7.060321420e-02f,-3.434713557e-02f},{-9.638720192e-03f,7.666251063e-02f,-1.954720356e-02f,-8.110293001e-02f,-9.334960487e-04f,7.036873698e-02f,-4.250418767e-02f,2.942128666e-02f,-5.609052628e-02f,1.332656946e-02f,5.012272298e-02f,-7.873966545e-02f,4.042444006e-02f,5.010301620e-02f,8.649681695e-03f,2.164967731e-02f,4.018452764e-02f,5.695264041e-02f,-4.983405024e-02f,-1.014169678e-01f,5.474784970e-02f,-3.983516246e-03f,-2.124105394e-02f,-1.140528247e-01f,8.496367186e-02f,-1.036458313e-01f,-4.010168370e-03f,9.975412861e-03f,-2.578623127e-03f,-7.039897889e-02f,-5.884677917e-02f,5.725545064e-02f,5.467937887e-02f,1.904374920e-02f,-3.977564350e-02f,6.135102734e-02f,1.682465896e-02f,-2.930608019e-02f,4.810090736e-02f,2.414012142e-02f,4.732359946e-02f,-1.039531454e-01f,1.270652935e-02f,2.587351017e-03f,5.323908478e-02f,5.527735688e-03f,-6.481729448e-02f,4.759685323e-02f,1.480051409e-02f,-2.065076120e-02f,2.302936837e-02f,-7.384955138e-02f,-6.028017774e-02f,-6.967142969e-02f,-7.301270962e-02f,-1.472266973e-03f,3.939466551e-02f,-4.242286086e-02f,-4.945507273e-02f,9.807449766e-03f,-5.116872024e-03f,3.049671650e-02f,7.708441466e-02f,4.407147691e-02f,2.380628511e-02f,-5.283854529e-02f,6.808950007e-02f,7.796967030e-02f,5.853053182e-02f,-6.993914023e-03f,-7.060747594e-02f,-8.803793788e-02f,-3.933605179e-02f,5.326616112e-03f,-6.942886394e-03f,-2.833233774e-02f,1.197954733e-02f,-1.756726950e-02f,-8.320022374e-02f,-2.297418891e-03f,-8.085003356e-05f,-5.464628339e-02f,-7.522211410e-03f,-7.388076931e-02f,-4.902929068e-02f,1.473189052e-02f,7.771632820e-02f,6.042528525e-02f,5.697138608e-02f,4.392332584e-02f,5.803529639e-04f,4.673832282e-02f,-3.105029836e-02f,2.915673703e-02f,1.421071403e-02f,-8.944349736e-02f,-4.102495033e-03f,-8.490763605e-02f,-8.609526604e-02f,5.675224960e-02f,-4.506919906e-02f,-7.412219048e-02f,3.651084751e-02f,-7.267591357e-02f,-4.736455157e-02f,-3.197920043e-03f,-5.790872499e-02f,2.071340010e-02f,-3.936982527e-02f,6.652832776e-02f,8.432898670e-02f,-5.317183584e-02f,6.064312905e-02f,9.581472725e-03f,7.837096602e-02f,-1.510220207e-02f,2.741608582e-02f,4.064615443e-02f,1.672420651e-02f,6.568866223e-02f,-1.244007796e-02f,-6.421070080e-03f,-2.678353712e-02f,-7.949435152e-03f,-8.703950793e-02f,1.106942073e-01f,-8.922030777e-02f,8.869920671e-02f,4.442863911e-02f,6.933211815e-03f,6.906845421e-02f,6.231487170e-02f,3.572313348e-03f,2.751694806e-02f,-7.850258052e-02f,6.001207232e-02f,1.981941424e-02f,-4.062787816e-02f,4.600137845e-02f,6.710819900e-02f,-1.285706833e-02f,-1.150503941e-02f,-1.014304459e-01f,-1.050323248e-02f,3.359513730e-02f,-7.959858514e-03f,2.261213958e-02f,-1.356444508e-01f,-1.219074130e-01f,-8.313956857e-02f,-1.007892005e-02f,3.834366426e-02f,2.698472934e-03f,-9.241870791e-02f,-3.115739487e-02f,1.231953409e-02f,-5.309837312e-02f,-1.813366450e-02f,-5.414769799e-02f,-8.198186755e-02f,-4.584136978e-02f,2.284101397e-02f,-3.064977750e-02f,5.682636425e-02f,4.355730861e-02f,-3.690319508e-02f},{4.279281944e-02f,-6.537172943e-02f,5.363661051e-02f,-9.272021800e-02f,3.495110571e-02f,9.925583377e-03f,-1.073450409e-02f,-8.489547670e-02f,-1.181010343e-02f,3.728108108e-02f,-6.705252081e-02f,-7.327587157e-02f,-1.185004879e-02f,5.914387852e-02f,2.899077535e-02f,-6.598992646e-02f,-5.132523552e-02f,-1.994559169e-02f,-5.802119523e-02f,1.029786654e-02f,-8.656039834e-02f,-1.554017887e-02f,5.804742873e-02f,-4.070998728e-02f,4.285570234e-02f,-6.851788610e-03f,2.994140610e-02f,4.938913509e-02f,-3.244700655e-02f,-2.066330239e-02f,-7.462276518e-02f,-2.607194148e-02f,4.680278525e-02f,6.982940435e-02f,-7.039217651e-02f,-4.809601232e-02f,-6.037709489e-02f,5.026806146e-02f,-4.250567779e-02f,5.531949922e-02f,-2.559662610e-02f,-5.415751040e-02f,-5.518862978e-02f,4.912741110e-02f,-2.143375389e-02f,4.087280855e-02f,2.735696547e-02f,1.656220481e-02f,-2.382013202e-02f,-8.408642374e-03f,-5.454838648e-02f,1.385279745e-02f,-3.325473517e-02f,-4.954221658e-03f,3.210826218e-02f,-3.825092688e-03f,6.520892680e-02f,5.241080653e-03f,-1.068506506e-03f,-3.205217794e-02f,3.721944243e-02f,2.245124988e-02f,7.168936729e-02f,3.202688321e-02f,-6.017883867e-02f,-1.231404487e-02f,-1.924189180e-02f,-2.107007988e-02f,-2.828498371e-02f,8.931428194e-02f,1.215307694e-02f,4.685194418e-02f,4.721325636e-02f,-3.656362742e-02f,-5.474218167e-03f,3.611452878e-02f,6.852000952e-03f,-4.330835119e-02f,4.787927121e-02f,-4.732295871e-02f,3.728854284e-02f,4.172157869e-02f,-2.197074145e-02f,-2.904861979e-02f,1.628060825e-02f,6.607465912e-03f,4.284181818e-02f,5.120878574e-03f,-3.460846096e-02f,-1.894268766e-02f,5.215141457e-03f,1.267729513e-02f,-1.875310391e-02f,-3.734157234e-02f,-5.629241001e-03f,3.615742084e-03f,-3.990403935e-02f,3.407481685e-02f,-6.338106096e-02f,-2.109093964e-02f,2.022870257e-02f,-6.109504774e-02f,5.299275741e-02f,4.969174042e-02f,-3.250151500e-02f,1.341713499e-02f,7.199341804e-02f,-3.015986644e-02f,-6.118058041e-02f,-2.034236304e-02f,-4.754161090e-02f,4.468054697e-02f,-3.419350088e-02f,4.853590578e-02f,-7.357193530e-02f,-8.758105338e-02f,-1.506153774e-02f,4.452053457e-02f,1.071525912e-05f,3.727848083e-02f,-2.599406429e-02f,2.872779220e-02f,-3.810547665e-02f,6.251011044e-02f,1.066257246e-03f,-3.605267033e-02f,-7.542616804e-04f,-8.728934452e-03f,8.530970663e-02f,-6.125544384e-02f,-2.854024991e-02f,-2.582438849e-02f,3.662904724e-02f,7.304514945e-02f,1.890318468e-02f,-6.853078306e-02f,-4.519747943e-02f,-4.855217785e-02f,1.593832858e-02f,7.586065680e-02f,-6.931395084e-02f,6.847257167e-02f,3.010386601e-03f,-2.528101020e-02f,6.127518415e-02f,2.186530456e-02f,3.989066929e-02f,-8.680739254e-02f,-8.111268282e-02f,-8.659108728e-02f,5.932201445e-02f,-3.330871230e-03f,-4.697419703e-02f,1.438356843e-02f,-5.456075445e-02f,-6.914725155e-02f,6.405783817e-03f,2.302034758e-02f,-3.547168896e-02f,-5.889075994e-02f,-5.011121929e-02f,-6.100113317e-02f,-9.328559041e-02f,2.983745374e-02f,-1.024647523e-02f,2.634513751e-02f},{-8.979230374e-02f,6.340356916e-02f,-2.354468219e-02f,-5.505385995e-02f,-8.555258811e-02f,-9.511623532e-03f,4.226058722e-02f,-6.046983413e-03f,-1.009140629e-02f,4.890929908e-02f,-3.385629877e-02f,-1.017419621e-03f,-1.061203657e-03f,-2.479728078e-03f,-3.029227257e-02f,1.568054780e-02f,4.823903367e-02f,2.650501393e-02f,2.269364521e-02f,3.859144449e-02f,-4.489335045e-02f,1.430520602e-02f,9.666287154e-02f,8.238543570e-02f,5.936525762e-02f,-1.097388100e-02f,3.830472380e-02f,4.044697061e-02f,-4.033899307e-02f,-5.570197478e-02f,2.790933102e-02f,-2.482569031e-02f,4.795256630e-02f,5.983007606e-03f,-4.519063979e-02f,-5.552370101e-02f,-2.198556811e-02f,-2.437018231e-02f,-2.500493266e-02f,4.518060386e-02f,2.108101547e-02f,7.071308047e-02f,-7.623979449e-02f,3.581497446e-02f,4.534037411e-02f,1.544111175e-03f,-3.637624159e-02f,2.171682566e-02f,-2.890142612e-02f,-3.146811947e-02f,9.439682215e-02f,3.798564523e-02f,1.476066280e-02f,-5.650158972e-02f,8.855962008e-02f,7.930128276e-02f,4.213251546e-02f,-4.767805338e-02f,2.339494973e-02f,1.177286543e-02f,-1.836317219e-02f,-3.589244932e-02f,3.006028011e-03f,6.491761655e-02f,3.768263385e-02f,6.523972005e-02f,7.330328226e-02f,6.938901544e-02f,-4.207375646e-02f,6.454133987e-02f,-6.500316411e-02f,-2.956221625e-02f,-1.658730581e-02f,1.224005595e-02f,-5.980766192e-02f,2.156744897e-02f,-1.593696699e-02f,4.342179373e-02f,1.152990852e-03f,4.978459701e-02f,-2.109866473e-04f,-5.684180558e-02f,4.572179914e-02f,-5.402163416e-02f,-5.142042041e-02f,-1.944053918e-02f,-2.249797992e-02f,-7.054211199e-02f,-5.032401532e-02f,-5.134865642e-02f,3.014400415e-02f,-8.498086780e-02f,-6.131636724e-02f,5.770324916e-02f,6.823001802e-02f,-5.414046347e-02f,-8.422412723e-02f,-1.340906881e-02f,-6.564830244e-02f,3.773930669e-02f,-8.165160567e-02f,4.845369235e-02f,-5.182768777e-02f,8.909424394e-02f,7.164335251e-02f,2.576649422e-03f,7.741107047e-02f,6.731029600e-02f,-5.582456663e-02f,4.418199509e-02f,-1.598143019e-02f,4.869648442e-02f,3.843123838e-02f,-1.289968286e-02f,2.520096488e-02f,2.981135435e-02f,5.553985946e-03f,-5.853616446e-02f,8.192916401e-03f,-6.522678584e-02f,4.033940285e-02f,7.271187007e-02f,5.347891152e-02f,-4.998565465e-02f,2.795077860e-02f,6.807207316e-02f,-4.707766324e-02f,4.344149679e-02f,3.729775921e-02f,7.648644969e-03f,1.176608726e-02f,3.368104994e-02f,5.377438664e-02f,-7.382761687e-03f,-6.322147697e-02f,-2.777742594e-02f,-5.136810616e-02f,6.822578609e-02f,-4.164503515e-02f,-6.249578437e-04f,1.553461887e-02f,7.483768463e-02f,-2.012901939e-02f,6.228412036e-03f,-5.080752075e-03f,-2.922660206e-03f,5.422119051e-02f,-9.596814960e-02f,-1.707253791e-02f,5.068999156e-02f,-7.835765369e-03f,1.528656110e-02f,7.306619734e-02f,-2.287990600e-02f,2.874989063e-02f,-6.317706406e-02f,-3.512132540e-02f,2.139801346e-02f,2.795199305e-02f,-2.144783363e-02f,-3.831903264e-02f,-9.477841109e-02f,-5.593501031e-03f,-1.649128087e-02f,-5.988653377e-02f,6.693953276e-02f},{2.467892878e-02f,-3.833980113e-02f,8.545015007e-03f,-6.447986513e-02f,1.019305214e-01f,-8.709757589e-03f,-2.304907888e-02f,1.085501164e-02f,4.364951327e-02f,-3.754491732e-02f,5.067871418e-03f,-8.219712973e-02f,-1.902420446e-02f,7.162576169e-02f,-2.787001058e-02f,-3.091308661e-02f,-4.030199349e-02f,-5.502988771e-02f,-1.393607445e-02f,-5.991661549e-02f,3.214687854e-02f,-8.009837568e-02f,6.399977952e-02f,1.534593874e-03f,5.129208416e-02f,-7.835194468e-03f,3.343756078e-03f,-2.768296003e-02f,7.598983496e-02f,-3.492995352e-02f,7.571016997e-02f,7.048549596e-03f,-5.064803734e-02f,1.330035459e-02f,-6.089423969e-02f,6.935334206e-02f,2.136287466e-02f,7.757799327e-02f,-3.118340485e-02f,-3.249103203e-02f,-3.655068204e-02f,-6.269872189e-03f,7.335741818e-02f,4.420529678e-02f,1.523283310e-03f,2.261820436e-02f,4.504146054e-02f,-9.096803516e-02f,1.058649179e-02f,1.568024233e-02f,-1.029469222e-01f,-1.194029301e-01f,3.552073985e-02f,2.648083493e-02f,8.122044057e-02f,5.812082067e-02f,-1.309197675e-02f,5.807520449e-02f,3.923277184e-02f,5.968470126e-02f,8.621339500e-02f,3.983944654e-02f,1.325223129e-02f,-8.329554647e-02f,8.724898845e-02f,-5.953954533e-02f,2.156268619e-02f,-1.045197155e-02f,-7.099205256e-02f,-2.697458491e-02f,1.874999516e-02f,-5.579030141e-02f,-3.676818218e-03f,4.502150416e-02f,-3.891522810e-02f,-3.387869895e-02f,6.398610305e-03f,4.104002938e-02f,-1.332918704e-01f,-9.043526649e-02f,-3.794415668e-02f,3.719835728e-02f,1.526583266e-02f,1.645850576e-02f,7.135498524e-02f,-3.715533763e-02f,-5.989539251e-02f,7.662760466e-02f,6.668831408e-02f,-3.660377488e-02f,-2.528248727e-02f,4.470290244e-02f,-3.609961644e-02f,-4.619724676e-02f,2.165080048e-02f,-8.164204657e-02f,7.945760339e-02f,3.415349871e-02f,-3.915669024e-02f,-7.049025595e-02f,2.925118804e-02f,5.934691802e-02f,-4.245127365e-02f,-7.404896617e-02f,-1.404637694e-01f,1.021560095e-02f,-5.785576254e-02f,-5.230019800e-03f,-5.931529403e-02f,-7.796750218e-02f,-5.166521668e-02f,-5.544929951e-02f,-6.552080065e-02f,-6.470879912e-02f,2.505385783e-03f,4.084539413e-02f,2.968597598e-02f,-8.941194415e-02f,-7.603833079e-02f,-4.399540275e-02f,7.059545815e-02f,4.483417422e-02f,8.320570737e-02f,5.922492221e-02f,-4.686101153e-02f,1.021708772e-01f,-7.454839069e-04f,-4.507151432e-03f,2.468386106e-02f,-2.577345446e-02f,4.787294660e-03f,7.569749840e-03f,-5.639234185e-02f,-6.271664798e-02f,-1.325375140e-01f,-1.624454372e-02f,2.824307792e-02f,-1.533368044e-02f,-8.596394211e-02f,-1.482260600e-02f,1.125885453e-02f,7.318613678e-02f,6.838613655e-03f,-5.535519868e-02f,-5.021142960e-02f,-5.238496512e-02f,-2.804942988e-02f,-9.023567289e-02f,-6.134794652e-02f,2.970569953e-02f,6.875576079e-02f,-1.983587630e-02f,3.367856145e-02f,-3.476964682e-02f,-2.797528915e-02f,-7.069893181e-02f,-5.174976215e-02f,-4.176434875e-02f,-5.499323830e-02f,-8.136136085e-02f,2.988963388e-04f,-4.176685959e-02f,3.448892618e-03f,1.225943398e-02f,-4.225204885e-02f,4.743507877e-02f},{-4.174831137e-02f,-1.578175463e-02f,-5.457244813e-02f,-1.003195066e-03f,2.888481319e-02f,4.154316336e-02f,-2.320917137e-02f,-6.072771549e-02f,-1.177175045e-01f,3.477186710e-02f,-5.231184512e-02f,-4.926912859e-02f,-7.921213657e-02f,-2.030073665e-02f,1.130702533e-02f,-9.780969471e-02f,3.009862266e-02f,2.801992185e-02f,6.827837229e-02f,3.409233689e-02f,8.601289243e-02f,2.375296131e-02f,-5.349992961e-02f,1.523943618e-02f,-6.264478713e-02f,-3.402446583e-02f,2.418794483e-02f,-4.244149104e-02f,-1.849225163e-02f,7.876385748e-02f,-7.318037003e-02f,-7.359894365e-02f,-1.117885336e-01f,4.609625787e-02f,-5.918410793e-02f,-4.556398094e-02f,-4.758181050e-02f,3.563660011e-02f,8.920788020e-02f,-8.675374091e-03f,5.159298331e-02f,1.002629027e-01f,9.660282172e-03f,2.734372206e-02f,-8.430757374e-02f,-4.208802804e-02f,2.836004831e-03f,8.999035694e-03f,-6.225318834e-02f,-1.787038706e-02f,6.882312894e-02f,-4.038604721e-02f,-7.202211767e-02f,-7.259018719e-02f,-4.771973938e-02f,-1.029609796e-02f,-9.638250805e-03f,-6.539706141e-02f,-1.123941764e-01f,7.572124898e-02f,-1.414100872e-03f,5.624927953e-02f,-4.091274738e-02f,8.292142302e-02f,5.752384290e-02f,-1.107813604e-02f,2.552841045e-02f,1.386133488e-02f,-7.692417502e-02f,-1.754442044e-02f,7.060409337e-02f,2.054621093e-02f,-7.338858396e-02f,-2.940703183e-02f,-3.547346592e-02f,-3.178913891e-02f,-3.712166101e-02f,-5.608943477e-02f,9.973769076e-03f,-9.035669267e-03f,-4.883008450e-02f,-6.376212090e-02f,-2.955291048e-02f,-5.612688884e-02f,-5.744423717e-03f,-1.147150435e-02f,-1.565507147e-03f,-9.033127688e-03f,-4.112570360e-02f,2.110582590e-02f,4.370723665e-02f,2.285754308e-02f,-3.265221417e-02f,-1.052162796e-01f,-4.272871092e-02f,-1.591138542e-02f,6.189727411e-02f,-5.058427528e-02f,-4.918213934e-02f,7.198792696e-02f,4.776735231e-02f,6.864029169e-02f,-1.953791827e-02f,-8.924897993e-04f,8.857686818e-02f,-3.161862493e-02f,2.220308594e-02f,-5.840035528e-02f,9.633838385e-02f,-5.719675869e-02f,6.142092496e-02f,-4.753489420e-02f,-1.669866964e-02f,-1.392197516e-02f,-4.005682096e-02f,-4.037192091e-02f,-7.727236487e-03f,4.414267465e-02f,-3.808978200e-02f,-2.729440294e-02f,3.200649284e-03f,8.553568041e-04f,-4.469937086e-02f,-4.205602407e-02f,-5.041214637e-03f,-2.941342257e-02f,1.839171397e-03f,-5.376281962e-02f,-4.663745686e-02f,-1.553463098e-02f,-1.880954020e-02f,-5.580066890e-02f,-1.000021398e-01f,2.158539183e-02f,-3.790205345e-02f,3.558728471e-02f,-6.207948085e-03f,1.838552579e-02f,-7.402753085e-02f,4.468883201e-02f,-9.229770512e-04f,-2.005886473e-02f,6.756410003e-02f,4.869964346e-02f,7.523697615e-02f,-2.414808422e-02f,9.746961296e-03f,-1.412126720e-01f,-1.142200977e-01f,4.884500429e-02f,6.559495628e-02f,5.755537190e-03f,5.010346323e-02f,-1.470225397e-02f,7.881122828e-02f,-5.694751441e-02f,-7.670063525e-02f,3.448600695e-02f,1.711218245e-02f,-1.434223820e-02f,-1.476295851e-02f,-6.337098032e-02f,5.984655023e-02f,6.101628765e-02f,-1.542420220e-02f,-9.301640093e-02f},{-2.816129290e-02f,3.796402365e-02f,-9.136013687e-02f,-2.862344868e-02f,-3.696738183e-02f,8.017864823e-02f,7.818621397e-02f,-7.573616691e-03f,1.042313054e-01f,4.181333631e-02f,4.296345264e-02f,-3.769714152e-03f,-4.993018508e-02f,-1.078186277e-02f,-6.588983536e-02f,2.094292082e-02f,3.586979210e-02f,-8.763515390e-03f,-1.843843726e-03f,9.416242130e-03f,-6.812072545e-02f,9.748703241e-02f,-2.963651158e-02f,-8.535430580e-02f,-2.953587286e-02f,1.330681238e-02f,-5.390381813e-02f,2.333835326e-02f,-2.484236844e-02f,6.351967901e-02f,-5.383108929e-02f,-6.386980414e-02f,2.931194194e-02f,-3.936823457e-02f,4.185945541e-02f,3.845674172e-02f,-2.831210382e-02f,4.413266480e-02f,9.431643784e-02f,7.608912885e-02f,-1.672354899e-02f,9.868728928e-03f,1.284596324e-01f,-2.686925791e-02f,1.258837408e-03f,7.094033062e-02f,-4.290614277e-03f,-3.792523220e-02f,-1.522554317e-03f,-1.889326796e-02f,6.485958397e-02f,8.142174035e-02f,-4.379356280e-02f,4.629055969e-03f,7.861319184e-02f,-4.075186327e-02f,4.357577115e-02f,-4.055809975e-02f,-3.241515532e-02f,-3.093419969e-02f,4.269544408e-02f,5.196594447e-02f,-6.181991473e-02f,6.770515442e-02f,-1.357537042e-02f,4.870241880e-02f,3.176872432e-02f,-8.394944482e-03f,-1.146718860e-02f,4.330958799e-02f,1.170820370e-02f,1.380685531e-02f,5.784807727e-02f,-6.167702377e-02f,1.032477617e-02f,1.005692873e-02f,5.039800331e-02f,-8.028997481e-02f,-5.537775159e-02f,-7.875494659e-03f,7.869786024e-02f,7.866100222e-02f,6.065071002e-02f,-3.635431081e-02f,9.911406040e-02f,-6.248021498e-02f,9.944513440e-02f,2.558884397e-02f,-5.383505672e-02f,1.735601388e-02f,1.001961716e-02f,4.071883485e-02f,2.479304932e-02f,1.178830955e-02f,5.082557723e-02f,1.858172752e-02f,9.082848579e-02f,-1.394333504e-02f,-3.211506084e-02f,6.143344939e-02f,-1.123433933e-02f,1.968621276e-03f,1.464168578e-01f,-5.243316665e-02f,-5.149883777e-02f,-5.845240504e-02f,-4.670774937e-02f,-8.110118099e-03f,-4.706470668e-02f,-4.243602604e-02f,-4.127981886e-02f,-3.054554947e-02f,-4.765789956e-02f,-4.521058127e-02f,-3.849812225e-02f,-2.877695719e-03f,1.403177250e-02f,-3.608463332e-02f,-7.411202788e-02f,3.702060878e-02f,9.148523211e-02f,1.552504022e-03f,-3.810082749e-02f,-4.463492706e-02f,-4.632510990e-02f,-4.580467194e-02f,7.105307281e-02f,-1.013588905e-02f,-3.141112998e-02f,7.517950982e-02f,-4.440888017e-02f,-1.150574256e-02f,5.183932558e-02f,6.000335142e-02f,-2.801603265e-02f,3.564814478e-02f,-4.494636506e-02f,-1.438281499e-02f,7.858458906e-02f,-5.783009902e-02f,1.241075806e-02f,-6.046830676e-03f,-1.344713848e-02f,7.207994908e-02f,-5.442259461e-02f,5.800388381e-02f,5.773033947e-02f,-1.168684885e-01f,-1.228029951e-01f,-1.116497163e-02f,9.958552197e-03f,5.445692316e-02f,-4.696308449e-02f,-2.926707268e-02f,-1.535574626e-02f,-1.713432558e-02f,-3.949603811e-02f,4.820114002e-02f,-1.464420278e-02f,2.729200944e-02f,2.167227119e-02f,-2.348806523e-02f,7.356934994e-02f,8.344043046e-03f,2.131206356e-02f,-3.161871061e-02f},{1.035450473e-01f,1.429797523e-02f,-1.926538069e-03f,4.420534149e-02f,-3.022467159e-02f,-2.256985754e-02f,-7.823262364e-02f,-7.814090932e-04f,3.559689969e-02f,-3.943352401e-02f,-5.426942836e-03f,-9.735755622e-02f,4.280211031e-02f,8.808900602e-03f,-2.518286183e-02f,-2.701818384e-02f,-7.926672697e-03f,-5.160836130e-02f,-5.304300413e-02f,-1.505437307e-02f,6.044853106e-02f,1.241501700e-02f,-2.259865869e-03f,-3.884492815e-02f,-4.505522549e-02f,5.902320147e-03f,3.611921892e-02f,2.557401545e-02f,-6.092262920e-04f,5.187132210e-02f,-6.205577776e-02f,-2.751264907e-02f,-2.772535570e-02f,4.629128799e-02f,3.389488533e-02f,-9.883235209e-03f,5.529290810e-02f,-2.137506194e-02f,2.097040974e-02f,3.320459276e-03f,-9.301225655e-03f,-1.736710034e-02f,3.684483841e-02f,2.217332646e-02f,-7.719846815e-02f,-1.637323201e-02f,-2.823164314e-02f,1.977959648e-02f,8.572438359e-02f,-8.256226778e-02f,2.598351799e-02f,-8.962875605e-02f,-1.584086940e-02f,5.027303845e-02f,8.153302222e-02f,-1.108550355e-01f,-5.227347836e-03f,-4.247592017e-02f,-3.168031201e-02f,-3.003854863e-02f,9.771199524e-02f,4.658639897e-03f,-5.301685631e-02f,-3.629857674e-02f,-3.705090284e-02f,2.452663891e-02f,-5.402828567e-03f,3.267792612e-02f,-3.029792570e-02f,6.871543825e-02f,1.872138679e-02f,1.486413740e-02f,1.982182451e-02f,-2.605893463e-02f,4.562380537e-02f,-5.505184643e-03f,-1.162443962e-02f,1.559862681e-02f,-4.285471886e-02f,-1.101443544e-02f,6.291151047e-02f,3.182589263e-02f,-7.082373649e-02f,1.719434746e-02f,6.752345711e-02f,7.296808995e-03f,-2.497367561e-02f,1.356381085e-02f,-7.905805111e-02f,-7.236892823e-03f,2.702986822e-02f,-2.478625067e-02f,-8.375472575e-02f,-1.781548560e-02f,9.703763574e-02f,-4.613213986e-02f,6.520356983e-02f,6.071914732e-02f,-4.973044991e-02f,2.250007540e-02f,-5.717296153e-02f,5.000124872e-02f,-4.437105358e-02f,4.162650555e-02f,-3.984056413e-03f,-4.650815576e-02f,8.029421419e-02f,-1.399330981e-02f,1.106387842e-02f,-4.134891182e-02f,-6.800014991e-03f,2.093549073e-02f,-1.569436863e-02f,-6.012104824e-02f,-5.011009425e-02f,-7.430034131e-02f,-1.499985252e-02f,-3.950857371e-02f,3.152263165e-02f,1.714691892e-02f,-3.288911283e-02f,-4.977891594e-02f,1.585041545e-02f,-5.993177649e-03f,5.097909644e-02f,1.048667803e-01f,9.136486053e-02f,-4.147395492e-02f,3.129065409e-02f,-5.628693104e-02f,5.065323785e-05f,-2.844480565e-03f,6.145001948e-02f,-4.147716612e-02f,-1.105605960e-01f,-6.997568160e-02f,2.986238711e-02f,-5.348041654e-03f,2.943248861e-02f,1.934014633e-02f,-2.223801985e-02f,-6.335627288e-02f,-7.621709257e-02f,9.738747962e-03f,-3.050961345e-02f,1.553242654e-02f,-4.435603693e-02f,2.481308999e-03f,-1.288203523e-02f,5.867253244e-02f,-1.327411178e-02f,4.941523075e-02f,-1.813713834e-02f,5.317281466e-04f,9.247846901e-02f,-8.036085218e-02f,-1.737278327e-02f,-1.067352146e-01f,-2.123644203e-02f,-2.298038639e-02f,-5.273390561e-02f,2.813838050e-02f,2.536873333e-02f,2.191160060e-02f,4.613276105e-03f,3.731265292e-02f},{-6.847448647e-02f,3.964433447e-02f,-1.619675756e-02f,-1.823015511e-02f,-5.769684166e-02f,7.327838242e-02f,4.114419688e-03f,7.675867062e-03f,-3.965521231e-02f,-1.666769758e-02f,-7.807850093e-02f,2.319802530e-02f,2.304105088e-02f,5.049958825e-02f,-4.420213029e-02f,-2.872855216e-02f,-3.995664418e-02f,-2.989113797e-03f,2.023848239e-03f,-4.953213036e-03f,-3.424682096e-02f,-3.732654452e-02f,-7.419408113e-02f,4.424560815e-03f,-3.811019659e-02f,-4.361973552e-04f,2.718813531e-02f,-5.045972392e-02f,-7.755234838e-02f,-3.241069987e-02f,-9.015604109e-02f,1.669710688e-02f,6.571907550e-02f,-4.342410341e-02f,-9.136641771e-02f,2.626339160e-02f,3.869339125e-03f,-8.946736157e-02f,2.585520735e-03f,-3.256872669e-02f,-1.050335988e-01f,-2.417378500e-02f,7.435657084e-02f,-5.646326020e-02f,-5.998755991e-02f,3.494249284e-02f,6.753993779e-02f,-3.830519319e-02f,6.542886794e-02f,9.252718650e-03f,5.582349747e-02f,1.752196066e-02f,-5.846212059e-02f,-3.674165905e-02f,-5.906800553e-02f,-6.288488209e-02f,4.620304331e-02f,-6.784094032e-03f,3.911778331e-02f,-8.706403524e-02f,3.030976839e-02f,-7.212912291e-02f,1.080337074e-02f,6.995513290e-02f,2.999412641e-02f,-7.883311063e-02f,-2.314692549e-02f,1.182704605e-02f,-3.304012120e-02f,8.284164965e-02f,-3.296679631e-02f,-1.043031290e-01f,1.922521368e-02f,3.661882505e-02f,-4.110972956e-02f,-6.171785668e-02f,5.892069638e-02f,1.837517694e-02f,-6.754691154e-02f,-9.296407551e-02f,3.750811517e-02f,-4.160998389e-02f,-4.649913311e-02f,-1.478764787e-02f,-1.157593634e-02f,-6.661421061e-02f,-9.411860257e-02f,1.817203313e-02f,6.765294354e-03f,-3.966452088e-03f,3.465864062e-02f,-1.148145832e-02f,5.872689560e-02f,-1.040676516e-02f,3.909217566e-02f,-2.295801975e-02f,7.731261104e-02f,8.683928102e-02f,6.744728424e-03f,4.217050225e-02f,-5.171399564e-02f,-2.254201844e-02f,-9.261768311e-03f,6.955415756e-02f,-5.448154360e-02f,4.391853884e-02f,1.877149940e-02f,-3.845354170e-02f,-4.499155283e-02f,1.091548335e-02f,-3.134832159e-02f,1.726238616e-02f,4.284486175e-02f,6.595360488e-02f,-5.124726892e-02f,3.425066173e-02f,-2.113248408e-02f,3.754821420e-02f,-1.043463349e-01f,-7.758435607e-02f,-1.162590273e-02f,6.020307913e-02f,4.921954498e-02f,5.460795388e-02f,1.469257474e-02f,3.789911047e-02f,-3.612814844e-02f,1.666022278e-02f,-6.120645255e-02f,-1.722883806e-02f,8.748675697e-03f,4.073167965e-02f,1.693248563e-02f,-4.619384184e-02f,4.756342247e-02f,-4.072143883e-02f,7.394591719e-02f,1.595281437e-02f,2.129855275e-04f,8.212859742e-03f,2.131649293e-02f,4.531779885e-02f,-1.547567397e-01f,2.464393328e-04f,7.215261459e-02f,6.240240484e-02f,9.219952859e-03f,1.073327498e-03f,-7.446392626e-02f,3.177362680e-02f,1.336564422e-01f,3.607797250e-02f,4.757515714e-02f,6.977056712e-02f,-5.273117125e-02f,-3.989074007e-02f,-1.128587946e-01f,-7.307632267e-02f,-2.939296886e-02f,-2.055179700e-02f,-1.002036035e-01f,4.425732419e-03f,-1.094554216e-01f,2.664225176e-02f,5.660542846e-02f,1.255635452e-02f},{-7.440592349e-02f,3.074134467e-03f,-5.267717689e-02f,5.234204233e-02f,-1.383791678e-02f,-1.820383780e-02f,-1.653838716e-02f,1.028850749e-01f,-4.081307352e-02f,9.028135240e-02f,2.094780654e-02f,-2.392637171e-02f,7.211159915e-02f,1.579635218e-02f,-5.400460958e-02f,7.447870076e-02f,5.810387060e-02f,-2.649540082e-02f,6.367315352e-02f,7.463659346e-02f,1.833339594e-02f,2.745833434e-02f,-7.443629950e-02f,-6.240735948e-02f,7.093370706e-02f,1.757095382e-02f,-3.083966859e-02f,6.960622221e-02f,4.647227004e-02f,8.902074397e-02f,-4.785564169e-02f,-4.334606603e-02f,-3.592935577e-02f,5.177348480e-02f,1.035447884e-02f,-4.796763137e-02f,-1.649929583e-02f,-7.700670511e-02f,1.743680798e-02f,-6.980068237e-02f,-2.287520561e-03f,5.619539879e-03f,1.759845950e-02f,5.942534283e-02f,-3.885872290e-02f,7.503793389e-02f,2.199435420e-02f,-4.077791050e-02f,-3.885625396e-03f,5.192846991e-03f,5.726457760e-02f,-2.209385112e-02f,5.060204864e-02f,-1.661485061e-03f,-6.985078752e-02f,6.910798699e-02f,2.560028341e-03f,6.178586930e-02f,9.558366984e-02f,7.581093907e-02f,-6.918260455e-02f,4.957640171e-02f,4.553747550e-02f,-2.882315405e-02f,1.044073887e-02f,-1.323208306e-02f,-6.101425737e-02f,6.274179369e-02f,8.839313686e-02f,-1.766677015e-02f,-2.161774412e-02f,3.403916582e-02f,4.305604845e-02f,1.624822058e-02f,7.813066989e-02f,8.097704500e-02f,-4.661599174e-02f,6.594133377e-02f,7.190196961e-02f,-2.026859671e-03f,7.282225043e-02f,5.901291966e-02f,-6.583185494e-02f,-5.644456297e-02f,-8.969852701e-03f,-4.559171945e-02f,5.843307823e-02f,3.445706889e-02f,5.876275524e-02f,4.726484418e-02f,3.046482243e-02f,2.886338532e-02f,-4.861999303e-02f,-5.889581144e-02f,-5.013772100e-02f,7.270228863e-02f,2.865606546e-02f,5.845927447e-02f,1.774992235e-02f,8.853496611e-02f,1.624002494e-02f,-1.059781536e-01f,6.390994415e-03f,-5.690358207e-02f,3.331970796e-02f,-2.925015055e-02f,-1.011077501e-02f,-4.989437759e-02f,-7.640641928e-02f,1.781296730e-02f,-2.036329359e-02f,7.540979236e-02f,-6.580055505e-02f,-5.765796639e-03f,-5.986825004e-02f,-2.474632487e-02f,7.619182020e-02f,-6.800282001e-02f,1.821233076e-03f,3.329373524e-02f,3.808449581e-02f,7.159150206e-03f,-2.845246904e-02f,-1.061654836e-01f,7.558916509e-02f,2.315546572e-02f,4.300092533e-02f,-1.188311167e-02f,-3.711959347e-02f,-5.991809815e-02f,-4.882879090e-03f,-3.498127684e-02f,-7.126966864e-02f,6.708691269e-02f,-3.713072091e-02f,-3.141043335e-02f,1.319933962e-02f,2.639143355e-02f,-4.640574008e-02f,-6.536017358e-02f,-6.061914191e-02f,6.580395252e-02f,-7.092464715e-02f,-5.221835524e-02f,1.300385688e-02f,4.602081329e-02f,3.499871865e-02f,1.699065231e-02f,3.485672921e-02f,-3.520804271e-02f,-4.287718609e-02f,2.706098557e-02f,3.549328446e-02f,-7.246953994e-02f,4.357824847e-02f,-1.328839175e-02f,-1.014532894e-02f,-3.058071807e-02f,2.013964951e-02f,-9.439473599e-02f,-1.133237928e-01f,-1.254552007e-01f,-2.650808357e-02f,5.162755400e-02f,-4.774858244e-03f,-3.728492185e-02f}};
static constexpr float bias[]={-1.695054024e-02f,1.733013056e-02f,-1.149041578e-01f,-4.336767644e-02f,-9.121303260e-02f,-1.808860898e-02f,-1.333367527e-01f,-4.290432483e-02f,-1.602421142e-02f,-7.595671341e-03f,2.480101585e-02f,2.128123119e-02f,-1.516321003e-01f,3.488187119e-02f,-2.726376057e-02f,-1.057754681e-01f,1.987091824e-02f,-8.674326539e-02f,-4.677524045e-02f,-5.945600942e-02f,5.560401827e-02f,-9.716582298e-02f,-1.040354297e-01f,-3.897134215e-02f};
static constexpr float out[]={1.113916095e-02f,6.697293371e-02f,4.492275789e-02f,-1.004046872e-01f,-2.080226131e-02f,-2.823547274e-02f,4.396497458e-02f,6.111349910e-03f,2.893270366e-02f,3.101087362e-02f,-3.334771842e-02f,7.085810415e-03f,6.403697282e-02f,-4.028788209e-02f,-5.055576190e-02f,-3.179816529e-02f,-1.868272782e-03f,2.079951763e-02f,-4.839031398e-02f,1.791236363e-02f,-8.152263239e-03f,-2.052752115e-02f,-4.780335724e-02f,2.312688716e-02f};
for(int i=0;i<24;++i){float q=bias[i];for(int j=0;j<inputWidth;++j)q+=h[i][j]*x[j];
v+=out[i]*(max(0.f,q)-max(0.f,bias[i]));}
return 64.*v;}
Candidate choosePaper(int original,const vector<Candidate>& made){
 vector<const Candidate*> options{&parents[original]};
 for(const auto& c:made)options.push_back(&c);
 int anchor=0;
 for(int i=1;i<int(options.size());++i)if(options[i]->price<options[anchor]->price)anchor=i;
 const auto& old=*options[anchor];const auto oldAggregate=aggregate(old);
 int winner=anchor;double best=-duelThreshold;
#ifdef BOUNDED_AUDIT
 cerr<<"[duel] anchor "<<old.mode<<' '<<old.slot<<'\n';
#endif
 for(int i=0;i<int(options.size());++i)if(i!=anchor){
  const auto& c=*options[i];const auto a=aggregate(c);
  const auto forward=compareFeatures(c,old,c.mode,a,oldAggregate);
  const auto reverse=compareFeatures(old,c,old.mode,oldAggregate,a);
  // 符号を逆にした対にも同じモデルを使い、同一紙同士の予測費用差は常に0にする。
  const double value=c.price-old.price+duelMix*.5*(duelModel(forward)-duelModel(reverse));
#ifdef BOUNDED_AUDIT
  cerr<<"[duel] diff "<<c.mode<<' '<<c.slot<<' '<<setprecision(17)<<value<<'\n';
  if(const char* p=getenv("BOUNDED_DUMP")){
   ofstream out(string(p)+"/"+to_string(10*c.mode+c.slot)+".duel",ios::binary);
   out.write(reinterpret_cast<const char*>(forward.data()),sizeof(forward));
   out.write(reinterpret_cast<const char*>(reverse.data()),sizeof(reverse));out.close();if(!out)abort();
  }
#endif
  if(value<best){best=value;winner=i;}
 }
 return *options[winner];
}

}

// 8試作から選択した紙の役割を保ち、離れた2枚の端点と参照棚の位置を交換する。
// 候補を同じ3つの人工棚で採点し、設計選択を計測時の負荷から独立させる。
static void endpointMarket(){
 constexpr int trials=8;
 array<Card,K> original;copy_n(cards,K,original.begin());
 const auto oldHome=to_array(home);
 array<U,K> body,ink;body.fill(~U(0));ink.fill(0);
 array<vector<int>,K> primary,clients;
 for(int a=0;a<P;a++){
  int top=core[a][0];primary[top].push_back(a);
  for(int k:core[a])clients[k].push_back(a);
 }
 for(int k=0;k<K-DonorCount;k++){
  ink[k]=A[primary[k][0]];
  for(int a:primary[k])body[k]&=~(A[a]^ink[k]);
 }
 auto render=[&](const array<int,K>& pos){
  for(int k=0;k<K-DonorCount;k++){int z=pos[k]/256;cards[k].m=(original[k].m&low)|(body[k]&lo(z+1)&~low);cards[k].end=z;}
  for(int a=0;a<P;a++)if(core[a].size()==2){
   int top=core[a][0],b=core[a][1];if(b<K-DonorCount)cards[b].m&=low|cards[top].m|~(A[a]^ink[b]);
  }
  for(int k=0;k<K-DonorCount;k++)cards[k].v=ink[k]&cards[k].m;
 };
#ifdef LOCAL
 // 端点を交換しないときは、8試作側が確定した全紙を再現できなければならない。
 render(oldHome);
 for(int k=0;k<K;++k){
  if(cards[k].m!=original[k].m || cards[k].v!=original[k].v || cards[k].end!=original[k].end)abort();
 }
 trace_stats.count("endpoint_identity_checks");
#endif
 array<array<uint8_t,K>,3> sampled;const auto& log=runningRequests->synthetic;
 for(int t=0;t<3;t++)sampled[t]=log[log.size()*(2*t+1)/6].stack;
 const auto initial=learnedShelf;
 struct Swap{int k,v;float value;};
 auto proxy=[&](int a){
  U cov=0;for(int k:core[a])cov|=cards[k].m;
  return float(staticCost(~cov,a))+.7f*pc(~cov);
 };
 auto roleDelta=[&](int k,int z,int depth){
  // The shape on the first card also controls the printable ink on its supports.
  int slope=0;for(int a:clients[k]){U other=0;for(int y:core[a])if(y!=k)other|=cards[y].m;
   U newly=lo(z+1)^lo(cards[k].end+1);
   slope+=pc(newly&~other&body[k]&~(ink[k]^A[a]));}
  return (z<cards[k].end?slope:-slope)*2+(depth-(home[k]%256))*int(clients[k].size())*3;
 };
 auto assigned=oldHome;
 vector<Swap> list;
 for(int k=0;k<K-DonorCount;k++)for(int v=0;v<k;v++){
  int x=cards[k].end,y=cards[v].end;if(x==y||abs(y-x)>14)continue;
  if(!(body[k]>>y&1)||!(body[v]>>x&1))continue;
  if((ink[k]^ink[v])&((U(1)<<x)|(U(1)<<y)))continue;
  int estimate=roleDelta(k,y,home[v]%256)+roleDelta(v,x,home[k]%256);
  list.push_back({k,v,float(estimate)});
 }
 stable_sort(list.begin(),list.end(),[](auto a,auto b){return a.value<b.value;});
 if(list.size()>72)list.resize(72);
 array<float,P> before;for(int a=0;a<P;a++)before[a]=proxy(a);
 auto affected=[&](int k,int v){
  vector<int> as=clients[k];as.insert(as.end(),clients[v].begin(),clients[v].end());
  size_t n=as.size();for(size_t i=0;i<n;i++)if(core[as[i]].size()==2){
   auto& others=clients[core[as[i]][1]];as.insert(as.end(),others.begin(),others.end());
  }
  sort(as.begin(),as.end());as.erase(unique(as.begin(),as.end()),as.end());return as;
 };
 for(auto& o:list){
  swap(assigned[o.k],assigned[o.v]);render(assigned);float optical=0;
  if(!(cards[o.k].m>>cards[o.k].end&1)||!(cards[o.v].m>>cards[o.v].end&1)){o.value=10000;}
  else{for(int a:affected(o.k,o.v))optical+=proxy(a)-before[a];o.value=.3f*o.value+optical;}
  swap(assigned[o.k],assigned[o.v]);render(assigned);
 }
 stable_sort(list.begin(),list.end(),[](auto a,auto b){return a.value<b.value;});
 if(int(list.size())>trials)list.resize(trials);
 auto price=[&](int a,int t,int k,int v){
  State start;start.a=sampled[t];
  if(k>=0){int i=locate(start,k),j=locate(start,v);if(i>18&&j>18)swap(start.a[i],start.a[j]);}
  int best=128;auto test=[&](State out){
   if(taildp.solve(out,A[a])){
    Sched sc;sc.run(start,out,2);best=min(best,int(out.op.size()));
   }
  };
  ++queryStamp;test(start);for(int r=0;r<int(core[a].size());r++){
   State s=start;if(fetchCore(s,core[a],r))test(std::move(s));
  }return best;
 };
 State rest;rest.a=initial;for(int i=0;i<K;i++)shelfRank[rest.a[i]]=i;
 shelfReady=true;pricing=GripPrice{};pricing.init();frozenGeometry=false;
 int memo[P][3];for(auto& r:memo)fill(begin(r),end(r),-1);
 for(auto& o:list){
  if(o.value>=9999)continue;
  LOCAL_ONLY(trace_stats.count("endpoint_priced_pairs"));
  auto as=affected(o.k,o.v);
  // Include non-clients that used one of these cards for a residual window.
  for(int t=0;t<int(log.size());t+=3){
   const auto& ex=log[t];
   if(((ex.moved[o.k/64]>>(o.k%64))|(ex.moved[o.v/64]>>(o.v%64)))&1)as.push_back(ex.product);
  }
  sort(as.begin(),as.end());as.erase(unique(as.begin(),as.end()),as.end());
  for(int a:as)for(int t=0;t<3;t++)if(memo[a][t]<0)memo[a][t]=price(a,t,-1,-1);
  swap(assigned[o.k],assigned[o.v]);render(assigned);int delta=0;
  for(int a:as)for(int t=0;t<3;t++)delta+=price(a,t,o.k,o.v)-memo[a][t];
  swap(assigned[o.k],assigned[o.v]);render(assigned);
  o.value=delta+0.18f*o.value;
 }
 stable_sort(list.begin(),list.end(),[](auto a,auto b){return a.value<b.value;});
 array<bool,K> taken{};int applied=0;vector<pair<int,int>> moves;
 for(auto o:list){
  if(o.value>=-3||applied>=5)break;
  if(taken[o.k]||taken[o.v])continue;
  swap(assigned[o.k],assigned[o.v]);render(assigned);
  if(!(cards[o.k].m>>cards[o.k].end&1) || !(cards[o.v].m>>cards[o.v].end&1) || !certified()){swap(assigned[o.k],assigned[o.v]);render(assigned);continue;}
  taken[o.k]=taken[o.v]=true;++applied;moves.emplace_back(o.k,o.v);
 }
 copy(assigned.begin(),assigned.end(),home);
 frozenGeometry=false;shelfReady=false;pricing=GripPrice{};fill_n(cacheStamp,K,0);queryStamp=0;taildp.resetGeometry();
 if(applied){
  array<int,K> occupant;for(int k=0;k<K;k++)occupant[k]=k;
  for(auto [k,v]:moves)swap(occupant[k],occupant[v]);
  State initial;initial.a=learnedShelf;for(auto& k:initial.a)k=occupant[k];
  auto next=make_shared<ShelfRequests>(initial);
  for(int i=0;i<K;i++)shelfRank[initial.a[i]]=i;shelfReady=true;pricing.init();frozenGeometry=true;taildp.resetGeometry();
  State previous=initial;int rebuilt=0;bool certificatesReady=false;
  for(int j=0;j<int(log.size());j++){
   const auto& ex=log[j];State after;after.a=ex.stack;
   for(auto [k,v]:moves){int p=locate(after,k),d=locate(after,v);if(p>16&&d>16)swap(after.a[p],after.a[d]);}
   // Use a transported observation only if its displayed product survives.
   // The available history records identities of moved sheets, not the old
   // script, so regenerate every fourth otherwise-valid example as well.
   if((j&3)&&display(after)==A[ex.product]){
    for(int k=0;k<K;k++)if((ex.moved[k/64]>>(k%64))&1)after.op.push_back({uint8_t(k),0});
   }else{
    previous.op.clear();previous.front=0;previous.cover=0;++queryStamp;int hi=10000;
    auto probe=[&](State x){if(taildp.solve(x,A[ex.product])&&int(x.op.size())<hi){Sched sc;sc.run(previous,x,2);hi=x.op.size();after=std::move(x);}};
    probe(previous);for(int r=0;r<int(core[ex.product].size());r++){
     State x=previous;if(fetchCore(x,core[ex.product],r))probe(std::move(x));
    }
    if(hi==10000){
     // This public design simulation occurs before main builds the final
     // retrieval certificates, so build them for the current ink on first use.
     if(!certificatesReady){makeRescues();certificatesReady=true;}
#ifdef TRANSPORT_CERT_AUDIT
     cerr<<"certified synthetic retrieval "<<j<<" product "<<ex.product<<'\n';
#endif
     after=previous;rescue(after,ex.product);
    }++rebuilt;
   }
   next->observe(after,ex.product);previous=std::move(after);
  }
  LOCAL_ONLY(trace_stats.count_by("endpoint_rebuilt_examples",rebuilt));
  next->arrange();learnedShelf=next->order;runningRequests=std::move(next);
  frozenGeometry=false;shelfReady=false;pricing=GripPrice{};queryStamp=0;fill_n(cacheStamp,K,0);taildp.resetGeometry();
#ifdef ENDPOINT_AUDIT
  cerr<<"transport recomputed "<<rebuilt<<'\n';
#endif
 }

 LOCAL_ONLY(
  trace_stats.count_by("endpoint_swaps",applied);
  int changed=0;
  for(int k=0;k<K;++k)changed+=cards[k].m!=original[k].m || cards[k].v!=original[k].v;
  trace_stats.count_by("endpoint_changed_papers",changed);
 );
#ifdef ENDPOINT_AUDIT
 cerr<<"market swaps "<<applied<<" best "<<(list.empty()?0:list[0].value)<<'\n';
#endif
}

// v5166_winner_catalog.inc
namespace winner_remake {
using bounded_design::Candidate;
Candidate manufacture(Candidate& parent,int mode,int slot,const array<U,P>& original,unsigned seed,const vector<int>& cheap){
 price_experiment::repair=mode<100?mode:0;price_experiment::share=mode>=100?mode-100:0;
 if(price_experiment::repair>=5||price_experiment::share>=5)bounded_design::observe(parent);
 for(int a=0;a<P;a++)A[a]=observed_sheets::remap(original[a],parent.order);
 bitMap=parent.order;setGroupCount(parent.groups);keep_rectangle=parent.groups==7;rng.x=seed;
 inheritedCount=parent.inherit;inheritedCodes=parent.words;geometryTie=parent.trial;++geometryEpoch;
 Candidate candidate;
 candidate.trial=parent.trial;candidate.groups=parent.groups;candidate.order=parent.order;
 candidate.inherit=parent.inherit;candidate.words=parent.words;
 try{
  design();
  if(certified()){
   array<uint8_t,K> shelf;shared_ptr<ShelfRequests> requests;auto stats=layoutFeatures(cheap,shelf,requests);
   auto xf=geometry_price::features(cards,A,shelf,sampledShelves,sampledProbe,rankLayout(stats),parent.groups);
   array<float,P> items;const double price=geometry_model::score(xf,items);
   bounded_design::record(candidate,mode,slot,shelf,requests,xf,items,price);
  }
 }catch(int){}
 price_experiment::repair=price_experiment::share=0;return candidate;
}
#ifdef V5178_EXACT
bool same(const Candidate& a,const Candidate& b){
 if(!a.valid||!b.valid)return false;
 const auto& p=a.design;const auto& q=b.design;
 if(p.groupCount!=q.groupCount||p.h!=q.h||p.uses!=q.uses||p.codes!=q.codes||p.rank!=q.rank||p.anchors!=q.anchors||p.map!=q.map)return false;
 for(int k=0;k<K;k++)if(p.sh[k].m!=q.sh[k].m||p.sh[k].v!=q.sh[k].v||p.sh[k].end!=q.sh[k].end)return false;
 if(a.shelf!=b.shelf||a.item!=b.item||a.price!=b.price||a.partner!=b.partner||a.backer!=b.backer)return false;
 const auto& x=*a.requests;const auto& y=*b.requests;
 if(x.group!=y.group||x.requested!=y.requested||x.local!=y.local||x.order!=y.order||x.anchor!=y.anchor||x.examples!=y.examples||x.synthetic.size()!=y.synthetic.size())return false;
 for(size_t i=0;i<x.synthetic.size();i++)if(x.synthetic[i].stack!=y.synthetic[i].stack||x.synthetic[i].moved!=y.synthetic[i].moved||x.synthetic[i].product!=y.synthetic[i].product)return false;
 return true;
}
#endif
// v5178_expensive_pairs.inc
Candidate decide(Candidate winner,const array<U,P>& original,unsigned seed,const vector<int>& cheap){
 int forced=5,only=0;
#ifdef V5168_AUDIT
 if(const char* v=getenv("V5168_PICK"))forced=atoi(v);
 if(const char* v=getenv("V5168_ONLY"))only=atoi(v);
 if(forced<0||forced>5||only<0||only>4)abort();
#endif
 auto& parent=bounded_design::parents[winner.slot];if(!parent.valid)abort();
#ifdef V5178_EXACT
 auto reconstructed=manufacture(parent,winner.mode,winner.slot,original,seed,cheap);
 cerr<<"[day] pair_reconstructed "<<same(winner,reconstructed)<<'\n';
#endif
 const auto start=clock();array<Candidate,5> menu;menu[0]=std::move(winner);const auto& incumbent=menu[0];
 vector<tuple<double,int,int>> pairs;
 for(int a=0;a<P;a++){int b=incumbent.partner[a];if(b<=a||incumbent.backer[a]<0||incumbent.backer[a]>=P||incumbent.backer[b]<0||incumbent.backer[b]>=P)continue;
  pairs.emplace_back(-double(incumbent.item[a])-incumbent.item[b],a,b);}
 sort(pairs.begin(),pairs.end());array<double,5> prices;prices.fill(numeric_limits<double>::infinity());prices[0]=incumbent.price;
 int available=1,chosen=0;double limit=prices[0]-6.;array<int,4> broken{};broken.fill(-1);
 for(int i=1;i<5&&i<=int(pairs.size());i++){
  if(only&&only!=i)continue;
  auto [price,a,b]=pairs[i-1];joint6064::forbiddenPair={a,b};
  menu[i]=manufacture(parent,incumbent.mode,incumbent.slot,original,seed,cheap);
  joint6064::forbiddenPair={-1,-1};if(!menu[i].valid)continue;
  if(menu[i].partner[a]==b)abort();available|=1<<i;prices[i]=menu[i].price;broken[i-1]=0;
  for(int v=0;v<P;v++)if(incumbent.partner[v]>v&&menu[i].partner[v]!=incumbent.partner[v])++broken[i-1];
  if(prices[i]<limit){limit=prices[i];chosen=i;}
 }
 if(forced!=5)chosen=menu[forced].valid?forced:0;
 LOCAL_ONLY(cerr<<"[day] winner_menu "<<incumbent.mode<<' '<<incumbent.slot<<' '<<chosen<<' '<<available;
            for(double p:prices)cerr<<' '<<setprecision(15)<<p;
            cerr<<' '<<double(clock()-start)/CLOCKS_PER_SEC<<'\n';
            cerr<<"[day] forbidden_pairs "<<pairs.size();for(int i=0;i<4;i++){
             if(i<int(pairs.size()))cerr<<' '<<get<1>(pairs[i])<<' '<<get<2>(pairs[i]);else cerr<<" -1 -1";cerr<<' '<<broken[i];}cerr<<'\n';);
 return std::move(menu[chosen]);
}
}

void selectDesign(TimeKeeper& time_keeper){
 array<U,P> orig;copy_n(A,P,orig.begin());
 vector<int> cheap(P);iota(cheap.begin(),cheap.end(),0);
 for(int j=P-1;j>0;j--)swap(cheap[j],cheap[rng(j+1)]);
 const unsigned seed=rng.x;
 double bestCost=numeric_limits<double>::infinity();
 bool found=false;
 AxisProposal mutation;
 struct Finalist{
  int id=-1;double cost=1e100;array<double,7> signals{};
 };
 array<Finalist,2> finalists;
 // Each head order shares its pair index between the two group counts.
 joint6064::reachPrices(false);rng.x=seed;
 const auto normal=jointWindowDifficulty();
 array<array<int,N>,5> orders{orderJointWindows(normal,0),orderJointWindows(normal,36871),{}};
 // Reversing the preliminary coordinates exposes a different set of easy pairs.
 for(int a=0;a<P;a++){
  U reverse=0;for(int j=0;j<N;j++)reverse|=(orig[a]>>j&1ULL)<<(N-1-j);
  A[a]=reverse;
 }
 rng.x=seed;
 orders[2]=orderJointWindows(jointWindowDifficulty(),0);
 for(int& j:orders[2])j=N-1-j;
 for(int trial=0;trial<int(orders.size());trial++){
  inheritedCount=0;
  if(trial>=3){
   if(!found)break;
   orders[trial]=mutation.construct(trial==3?12:18);
  }
  const auto& order=orders[trial];geometryTie=trial;
  array<U,P> transformed{};
  for(int a=0;a<P;++a)for(int j=0;j<N;++j)transformed[a]|=((orig[a]>>order[j])&1ULL)<<j;
  for(int groups : {7,6}){
   // Use the second group count on the original coordinate system. The four
   // remaining coordinate systems each contribute one seven-group design.
   if(groups==6&&trial!=0)continue;
   copy_n(transformed.begin(),P,A);bitMap=order;
   setGroupCount(groups);keep_rectangle=groups==7;rng.x=seed;
   ++geometryEpoch;
   try {
    design();
    if(!certified())continue;
    array<uint8_t,K> shelf;shared_ptr<ShelfRequests> observations;
    const auto features=layoutFeatures(cheap,shelf,observations);
    double layout=rankLayout(features);
    auto xf=geometry_price::features(cards,A,shelf,sampledShelves,sampledProbe,layout,groups);
    array<float,P> items;double cost=geometry_model::score(xf,items);
    int id=trial==0?int(groups==6):trial+1;auto& parent=bounded_design::parents[id];
    parent.trial=trial;parent.groups=groups;parent.order=order;parent.inherit=inheritedCount;parent.words=inheritedCodes;
    bounded_design::record(parent,0,id,shelf,observations,xf,items,cost);
#ifdef DAY_GEOM_AUDIT
    cerr<<"design "<<trial<<" "<<groups<<" "<<cost<<" "<<time_keeper.exact_elapsed_sec()<<'\n';
#endif
    if(cost<bestCost){
     bestCost=cost;found=true;mutation.observe();
    }
    auto& f=finalists[trial>=3];
    if(cost<f.cost){
     double total=0,prep=0,hard=0;
     for(auto row:sampledProbe){total+=row[0];prep+=row[1];hard+=max(0.f,row[0]-12);}
     f.signals={cost,layout,total/P,prep/P,hard/P,features[6],features[9]};
     f.cost=cost;f.id=id;
    }
   } catch(int){}
  }
 }
 if(found){
  // Fit to forced normal/grown pairs on independent catalogs. Candidate
  // generation always follows the original score so the learned decision
  // sees the same six geometries as the production-label experiment.
  constexpr array<double,7> weights={0.694486439622,0.24919333906,133.593862956,-50.2499694951,288.345033421,55.9802631504,-36.1489228801};
  double advantage=1.17445530369;
  for(int i=0;i<7;i++)advantage+=weights[i]*(finalists[1].signals[i]-finalists[0].signals[i]);
  int take=finalists[1].id>=0&&(finalists[0].id<0||advantage<0);
  int original=finalists[take].id;auto& parents=bounded_design::parents;
  vector<bounded_design::Candidate> made;made.reserve(bounded_design::budgetLimit);
  int attempted=0,formed=0;const auto proposals=bounded_design::propose(original);
  for(auto proposal:proposals){
   auto& parent=parents[proposal.parent];int mode=proposal.mode;++attempted;
   price_experiment::repair=mode<100?mode:0;price_experiment::share=mode>=100?mode-100:0;
   if(price_experiment::repair>=5||price_experiment::share>=5)bounded_design::observe(parent);
   auto& order=parent.order;
   for(int a=0;a<P;++a)A[a]=observed_sheets::remap(orig[a],order);
   bitMap=order;setGroupCount(parent.groups);keep_rectangle=parent.groups==7;rng.x=seed;
   inheritedCount=parent.inherit;inheritedCodes=parent.words;geometryTie=parent.trial;++geometryEpoch;
   try{
    design();if(!certified())continue;++formed;
    array<uint8_t,K> shelf;shared_ptr<ShelfRequests> requests;
    auto stats=layoutFeatures(cheap,shelf,requests);
    auto xf=geometry_price::features(cards,A,shelf,sampledShelves,sampledProbe,rankLayout(stats),parent.groups);
    array<float,P> items;double price=geometry_model::score(xf,items);
    bounded_design::Candidate candidate;
    bounded_design::record(candidate,mode,proposal.parent,shelf,requests,xf,items,price);
    made.push_back(std::move(candidate));
   }catch(int){}
  }
  price_experiment::repair=price_experiment::share=0;
  auto winner=bounded_design::choosePaper(original,made);
  winner=winner_remake::decide(std::move(winner),orig,seed,cheap);
  LOCAL_ONLY(trace_stats.count_by("design_cost_hits",designCacheHits));
  LOCAL_ONLY(trace_stats.count_by("extra_attempted",attempted);trace_stats.count_by("extra_formed",formed);
             trace_stats.count_by("extra_selected",int(winner.mode!=0)));
#ifdef BOUNDED_AUDIT
  cerr<<"[day] live "<<original<<" "<<winner.mode<<" "<<winner.slot<<" "<<attempted<<" "<<formed<<"\n";
#endif
  winner.design.load();learnedShelf=winner.shelf;runningRequests=std::move(winner.requests);
  endpointMarket();
  crossEndpointShelf(learnedShelf,runningRequests->synthetic);haveLearnedShelf=true;
 }
 else{copy_n(orig.begin(),P,A);iota(bitMap.begin(),bitMap.end(),0);backupDesign();}
 rng.x=seed;
}
// 次注文以降の操作数を線形推定する。係数は独立人工注文による継続比較で学習済み。
// 物理取得深さは、対象紙の各印字窓について、それより前の印字紙枚数を数えた最小値とする。
struct FastValue{
 // Ridge correction for discounted continuation costs of paired synthetic terminal shelves.
 static constexpr double coef[119]={0.0136075149531,-0.000277322271748,0.0031576551776,0.00516640067023,0.0078382065868,0,0,0.00417420671457,-0.00995207083131,0.0136075149531,0.00579413907713,0.00599735302393,0.000840989837507,0.00776411581581,0.012927343194,0.00656677427555,0.00716942848619,0.00160849301203,-0.00328498252389,-0.000319541014617,0.00174320909532,0,0,0.0128011028878,-0.00827049946771,0.00716942848619,0.00295739070645,-0.00107205463404,-0.00886422740061,-0.0039569109375,0.010976677928,0.000657002150343,0.00769542281543,0.00171645019209,-0.00436561292437,0.000547913498161,0.000454544731371,0,0,0.0102692597802,-0.00989720381625,0.00769542281543,0.000106973855416,-0.00222869348844,-0.00352208404062,-0.0037378469004,0.0430765039968,-0.000819908035254,0.00766371963534,-0.00254293508042,-0.000936729700323,-0.00677317398771,-0.0030714155487,0,0,0.00880941856867,-0.00732689106631,0.00766371963534,-0.00113279784332,-0.00870653177095,0.00516095925633,-0.0100428344055,0.149309356031,0.00249871393102,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.10783134441,-0.00258408708043,-0.0013810909,0.148588667266,0,0.00339840273704,0.00422900468205,0.010783329924,-0.0002084637837,0,0};
 static constexpr array<double,11> landing{0.70880979,0.013966162,-0.0013163794,0.0070194225,0.0044959001,-0.01944488,0.28439541,0.77461735,-0.173862,-0.1164169,0.034528906};
 static constexpr double hiddenErrorPrice=0.458421180549;
 array<array<array<U,4>,2>,N> disagrees{};
 int homeRank[K]{},primary[K]{},secondary[K]{};
 double byDepth[K][18],byPosition[K][K],byVis[4][65],byEnd[4][18];
 int bin[K],col[K],c2[K];const double*w;
 void init(const State& s){
  for(int a=0;a<P;a++){
   U need=~U(0);for(int k:core[a])need&=~cards[k].m;
   for(;need;need&=need-1){int z=ctz(need);disagrees[z][1-((A[a]>>z)&1)][a/64]|=U(1)<<(a%64);}
  }
  w=coef;for(int i=0;i<K;i++)homeRank[s.a[i]]=i;
  for(auto &v:core)if(!v.empty()){primary[v[0]]++;if(v.size()>1)secondary[v[1]]++;}
  for(int k=0;k<K;k++){
   int l=cards[k].end;int b=bin[k]=min(3,max(0,(l-H)*4/max(1,N-H))),o=16*b;
   col[k]=int(cards[k].v>>l&1);c2[k]=(l&&(cards[k].m>>(l-1)&3)==3)?int(cards[k].v>>(l-1)&3):-1;
   bool full=(cards[k].m&low)==low;
   for(int d=0;d<=17;d++){
    byDepth[k][d]=w[o]*(d==0)+w[o+1]*min(d,3)+w[o+2]*min(d,8)+w[o+3]*min(d,15)
       +(w[o+11]*primary[k]+w[o+12]*secondary[k]+w[o+13]*full)*min(d,10)
       +(w[114]*(primary[k]>0)+w[115]*(secondary[k]>0))*(d==0)+w[116]*min(d,12)*usage[k];
   }
   for(int i=0;i<K;i++){
    int change=i-homeRank[k],d=abs(change);
    double correction=(landing[6]*d+landing[7]*min(d,24)+landing[8]*max(0,change)*primary[k]-landing[9]*cards[k].end*change)/K;
    byPosition[k][i]=w[o+7]*min(d,20)+w[o+8]*max(0,min(20,change))+correction/landing[0];
   }
  }
  for(int b=0;b<4;b++){
   for(int x=0;x<=64;x++)byVis[b][x]=w[16*b+9]*(x>0)+w[16*b+10]*x*x+w[110]*(x>0)+w[112]*x;
   for(int d=0;d<=17;d++)byEnd[b][d]=w[16*b+4]*min(12,d);
  }
 }
AHC_ARCH  double eval(const State&s)const{
  // After moving the first owner, a second black cell is visible only if
  // the second printed sheet is black. An uncovered window defaults to white.
  U once=0,twice=0,secondBlack=0,firstBlack=0;int secondOwners=0;
  for(int k:s.a){
   U second=cards[k].m&once&~twice;firstBlack|=cards[k].v&~once;
   secondOwners+=bool(second&(firstBlack^cards[k].v));secondBlack|=cards[k].v&second;
   twice|=second;once|=cards[k].m;if(twice==~U(0))break;
  }
  int persistentErrors=0;
  U b0=0,b1=0,b2=0,b3=0,covered=0;unsigned char d1[N][2],d2[N][4];
  memset(d1,255,sizeof(d1));memset(d2,255,sizeof(d2));double val=0;int prev=0;
  double optical=landing[4]*pc((firstBlack^secondBlack)&twice)+landing[5]*secondOwners;
  for(int i=0;i<K;i++){
   int k=s.a[i],l=cards[k].end,b=bin[k],d=0,e=0;U m=cards[k].m,t=m;
   U z=t&~b3;if(z)t=z;else d=8;z=t&~b2;if(z)t=z;else d+=4;
   z=t&~b1;if(z)t=z;else d+=2;d+=!(t&~b0);
   U bit=1ULL<<l;e=bool(b0&bit)+2*bool(b1&bit)+4*bool(b2&bit)+8*bool(b3&bit);
   U visible=m&~covered;int x=pc(visible);covered|=m;
   if(U same=visible&~(cards[k].v^secondBlack)){
    array<U,4> affected{};
    do{int z=ctz(same);auto& items=disagrees[z][(cards[k].v>>z)&1];
     for(int j=0;j<4;j++)affected[j]|=items[j];
    }while((same&=same-1));
    for(U items:affected)persistentErrors+=pc(items);
   }
   val+=byDepth[k][min(17,d)]+byPosition[k][i]+byVis[b][x]+byEnd[b][min(17,e)];
   if(x){
    val+=w[111]*bool(visible&low)+w[113]*min(16,abs(homeRank[k]-i));
    optical+=landing[10]+(landing[1]*primary[k]+landing[2]*secondary[k])*x+landing[3]*i/32.;
   }
   val+=w[108]*(l<prev)+w[109]*abs(l-prev);prev=l;
   d1[l][col[k]]=min(int(d1[l][col[k]]),d);
   if(c2[k]>=0)d2[l][c2[k]]=min(int(d2[l][c2[k]]),d);
   m&=~(b0&b1&b2&b3);
   U carry=b0&m;b0^=m;m=b1&carry;b1^=carry;carry=b2&m;b2^=m;b3^=carry;
  }
  for(int l=0;l<N;l++){
   int o=16*min(3,max(0,(l-H)*4/max(1,N-H)));
   for(int x:d1[l])if(x!=255)val+=w[o+14]*min(x,15);
   for(int x:d2[l])if(x!=255)val+=w[o+15]*min(x,15);
  }
  return (landing[0]*(val+hiddenErrorPrice*persistentErrors/P)+optical)/.75;
 }
} fv;


bool validPlan(const State& initial,const State& plan,U target){
  if(plan.op.size()>2*K)return false;
  auto order=initial.a;
  for(auto op:plan.op){
    int id=op.k,d=op.d;if(id>=K||d>=K)return false;
    int p=0;U cover=0;while(p<K&&order[p]!=id)cover|=cards[order[p++]].m;
    if(p==K||!(cards[id].m&~cover))return false;
    if(d<p)memmove(order.data()+d+1,order.data()+d,p-d);
    if(d>p)memmove(order.data()+p,order.data()+p+1,d-p);
    order[d]=id;
  }
  return order==plan.a&&display(plan)==target;
}

// 原文の復旧経路は、元の先頭紙を整列済み末尾へ順に挿入する。
// 全紙が端点へ印字するため、このK操作は全て合法となる。
void resetDisplay(State& s,int a){
 LOCAL_ONLY(trace_stats.count("resetDisplay_calls");trace_stats.mark_fallback());
 array<int,K> rank;rank.fill(-1);int n=0;
 for(int k:rescueLists[a])if(rank[k]<0)rank[k]=n++;
 for(int k:s.a)if(rank[k]<0)rank[k]=n++;
 s.op.clear();
 for(int unprocessed=K-1;unprocessed>=0;unprocessed--){
   int k=s.a[0],d=unprocessed;
   while(d<K-1&&rank[s.a[d+1]]<rank[k])d++;
   moveCard(s,0,d);
 }
}

// 候補最終順の中で1紙を動かすと、交差ごとに取得深さが変わるのは自身と隣紙だけとなる。
// 色ごとの最小値は、端点と局所模様ごとの16個の計数で維持する。
struct DepthImprove {
 // Fitted on independently generated public tables. Each target compared
 // two equal-cost legal placements under four matched 32-order continuations.
 static constexpr double terminalWeight[20]={0.621929232,0.000745169334,-0.00162567101,0.000607825973,-0.000289393618,0.00227306499,-0.00253940333,0.00156189909,0.00383778015,0.0417476506,-0.00217983407,-0.00283541945,-0.00236650371,0.00469310117,-0.00049737235,0.00642199535,-0.0265973768,0.00084278584,-0.00281701266,0.00466718725};
 int role(int k)const{
  if(k>=K-DonorCount)return 3;
  return fv.primary[k]>1?2:fv.secondary[k]?1:0;
 }
 double learnedCross(int k,int y,int pos,int dx,int dy,U wx,U wy,const double* w=terminalWeight)const{
  int hx=!(wx&~cards[y].m),hy=!(wy&~cards[k].m),rx=role(k),ry=role(y);
  double sx=1./pc(wx),sy=1./pc(wy);
  double x=hx*(w[2+rx]+w[6+rx]*(dx>=3)+w[14]*usage[k]+w[16]*(dx==0)+w[17]*pricing.use[k]/32.);
  double z=hy*(w[2+ry]+w[6+ry]*(dy>=3)+w[14]*usage[y]+w[16]*(dy==0)+w[17]*pricing.use[y]/32.);
  U diff=cards[k].v^cards[y].v;
  return x-z+w[1]*pricing.price(k,y)/64.+w[10+rx]*(fv.homeRank[k]<=pos?1:-1)-w[10+ry]*(fv.homeRank[y]<=pos?1:-1)
       +w[15]*(hx*pc(diff&wx)*sx-hy*pc(diff&wy)*sy)+w[18]*(pc(wx&cards[y].m)*sx-pc(wy&cards[k].m)*sy)
       +w[19]*((fv.homeRank[k]<fv.homeRank[y])-(fv.homeRank[k]>fv.homeRank[y]));
 }

 int nm=10,range=12,attempts=4,regular=3,capBlock=2,extra=0;
 struct BitsCount{
  U counters[8]{},cover=0;
  void add(U m){cover|=m;for(int j=0;m&&j<8;j++){U c=m&counters[j];counters[j]^=m;m=c;}}
  void sub(U m){for(int j=0;m&&j<8;j++){U c=m&~counters[j];counters[j]^=m;m=c;}cover=0;for(U z:counters)cover|=z;}
  void query(U m,int end,int &d,int &e,U &which)const{
   d=e=0;which=m;
   for(int j=7;j>=0;j--){U z=which&~counters[j];if(z)which=z;else d+=1<<j;e|=((counters[j]>>end)&1)<<j;}
  }
 };
 struct Hist{
  unsigned char count[16]{},m=15;
  void add(int d){d=min(d,15);count[d]++;m=min<int>(m,d);}
  int change(int from,int to){from=min(15,from);to=min(15,to);if(from==to)return 0;int old=m;
   count[from]--;count[to]++;if(to<m)m=to;else while(m<15&&!count[m])m++;return int(m)-old;}
 };
 double exposure(int k,int p,U v)const{int b=fv.bin[k],x=pc(v);return fv.byVis[b][x]+(x?fv.w[111]*bool(v&low)+fv.w[113]*min(16,abs(fv.homeRank[k]-p)):0);}
 struct BaseIndex{BitsCount pre[K+1];Hist h1[N][2],h2[N][4];int depth[K],pos[K];U under[K],visible[K];};
 void index(const State&F,BaseIndex&out)const{
  U back=0;
  for(int i=K-1;i>=0;i--){int k=F.a[i];out.under[k]=back;back=(back&~cards[k].m)|cards[k].v;}
  for(int i=0;i<K;i++){int k=F.a[i],d,e;U w;auto &pr=out.pre[i];pr.query(cards[k].m,cards[k].end,d,e,w);
   out.h1[cards[k].end][fv.col[k]].add(d);if(fv.c2[k]>=0)out.h2[cards[k].end][fv.c2[k]].add(d);
   out.depth[k]=d;out.pos[k]=i;out.visible[k]=cards[k].m&~pr.cover;out.pre[i+1]=pr;out.pre[i+1].add(cards[k].m);
  }
 }
 struct Choice{int k,d;double gain;};
 pair<int,double> locateBest(const State&F,int k,U target,const BaseIndex&base,vector<Choice>& choices){
  int old=base.pos[k],lx=cards[k].end,bx=fv.bin[k],oldDepth=base.depth[k];U mx=cards[k].m;
  U back=(target&~base.visible[k])|(base.under[k]&base.visible[k]);int bestD=old;double best=0;
  auto itemAt=[&](int p){return F.a[p+(p>=old)];};
  auto edge=[&](int a,int b){return b<0?0.:fv.w[108]*(a>b)+fv.w[109]*abs(b-a);};
  for(int dir:{-1,1}){
   Hist h1[N][2],h2[N][4];memcpy(h1,base.h1,sizeof(h1));memcpy(h2,base.h2,sizeof(h2));BitsCount now=base.pre[old];
   int d=old;double val=0,learned=0;
   while(abs(d-old)<range && d+dir>=0 && d+dir<K){
    int t=dir>0?d:d-1,y=itemAt(t),ly=cards[y].end,by=fv.bin[y];U my=cards[y].m;
    if(dir<0)now.sub(my);
    int dx,ex,dy,ey;U wx,wy;now.query(mx,lx,dx,ex,wx);now.query(my,ly,dy,ey,wy);
    int dx2=dx+!(wx&~my),dy1=dy+!(wy&~mx),ex2=ex+int(my>>lx&1),ey1=ey+int(mx>>ly&1);
    double delta=fv.byDepth[k][min(dx2,17)]-fv.byDepth[k][min(dx,17)]+fv.byDepth[y][min(dy,17)]-fv.byDepth[y][min(dy1,17)];
    delta+=fv.byEnd[bx][min(ex2,17)]-fv.byEnd[bx][min(ex,17)]+fv.byEnd[by][min(ey,17)]-fv.byEnd[by][min(ey1,17)];
    delta+=fv.byPosition[k][t+1]-fv.byPosition[k][t]+fv.byPosition[y][t]-fv.byPosition[y][t+1];
    U free=~now.cover;delta+=exposure(k,t+1,mx&free&~my)+exposure(y,t,my&free)-exposure(k,t,mx&free)-exposure(y,t+1,my&free&~mx);
    int pr=t?cards[itemAt(t-1)].end:0,nx=t+1<K-1?cards[itemAt(t+1)].end:-1;
    delta+=edge(pr,ly)+edge(ly,lx)+edge(lx,nx)-edge(pr,lx)-edge(lx,ly)-edge(ly,nx);
    val+=dir*delta;learned+=dir*learnedCross(k,y,t,dx,dy,wx,wy);
    auto ch=[&](Hist& hist,int x,int v){return dir>0?hist.change(x,v):hist.change(v,x);};
    val+=fv.w[bx*16+14]*ch(h1[lx][fv.col[k]],dx,dx2)+fv.w[by*16+14]*ch(h1[ly][fv.col[y]],dy1,dy);
    if(fv.c2[k]>=0)val+=fv.w[bx*16+15]*ch(h2[lx][fv.c2[k]],dx,dx2);
    if(fv.c2[y]>=0)val+=fv.w[by*16+15]*ch(h2[ly][fv.c2[y]],dy1,dy);
    if(dir>0)now.add(my);d+=dir;
    if((dir>0?dx2:dx)<=oldDepth+capBlock){
      U aff=mx&~now.cover;if(((back&~aff)|(cards[k].v&aff))!=target)continue;
      double q=terminalWeight[0]*val+learned+regular*.001*abs(d-old);
      if(q<-1e-7)choices.push_back({k,d,-q});
      if(q<best-1e-9){best=q;bestD=d;}
    }
   }
  }
  return {bestD,-best};
 }
 template<class Time>void improve(const State&start,State&res,U target,int item,const Time&room){
  if(!nm||!room(.000045))return;
  vector<int>ks;bool used[K]{};for(auto op:res.op)if(!used[op.k]){used[op.k]=true;ks.push_back(op.k);}
  if(ks.empty())return;
  if(nm<(int)ks.size()){int p=(item+start.a[0]+res.a[3])%ks.size();rotate(ks.begin(),ks.begin()+p,ks.end());ks.resize(nm);}
  vector<Choice> opts,choices;choices.reserve(128);
  BaseIndex base;index(res,base);
  for(int k:ks){if(!room(.000015))break;auto [d,g]=locateBest(res,k,target,base,choices);if(g>1e-7)opts.push_back({k,d,g});}
  sort(opts.begin(),opts.end(),[](auto&x,auto&y){return x.gain>y.gain;});
  // Disjoint inversion intervals preserve the set of prefix sheets before
  // the other insertion. Try realizing their combined layout at the same cost.
  if(opts.size()>1&&room(.000020)){
   // All alternatives for a sheet include its original position. Disjoint
   // interval scheduling therefore also enforces at most one move per sheet.
   int head[K],link[256],last[K+2];double value[K+2]{};
   fill_n(head,K,-1);fill_n(last,K+2,-1);
   for(int i=0;i<int(choices.size());i++){
    auto q=choices[i];int h=max(base.pos[q.k],q.d);link[i]=head[h];head[h]=i;
   }
   for(int h=0;h<K;h++){
    last[h+2]=last[h+1];value[h+2]=value[h+1];
    for(int i=head[h];i>=0;i=link[i]){
     auto q=choices[i];int lo=min(base.pos[q.k],q.d);
     double score=value[lo]+q.gain;
     if(score>value[h+2]){value[h+2]=score;last[h+2]=i;}
    }
   }
   State together=res;int n=0;
   for(int i=last[K+1];i>=0;){
    auto q=choices[i];int pos=base.pos[q.k];auto& a=together.a;
    if(q.d<pos)move_backward(a.begin()+q.d,a.begin()+pos,a.begin()+pos+1);
    else move(a.begin()+pos+1,a.begin()+q.d+1,a.begin()+pos);
    a[q.d]=q.k;++n;i=last[min(pos,q.d)];
    LOCAL_ONLY(trace_stats.count_by("shelf_dp_nonlocal",none_of(opts.begin(),opts.end(),[&](auto e){return e.k==q.k&&e.d==q.d;})));
   }
   if(n>1&&display(together)==target){
    Sched opt;opt.init(start,together);State result;
    for(int r=0;r<attempts;r++)if(opt.attempt(result,r,int(res.op.size())+1)){
     LOCAL_ONLY(trace_stats.count("combined_parking_orders");trace_stats.count_by("combined_parking_papers",n));
     res=std::move(result);return;
    }
   }
  }
  if(!choices.empty()&&room(.00003)){
   // Train and deploy the comparison on the same near-optimal menu. The
   // broad model still proposes displacements and handles disjoint batches.
   static constexpr double rerank[20]={1.17691035,0.00408947844,0.001404248,-0.00156929984,0.00159561107,0.0158476098,-0.00347482579,0.017993502,0.0114022298,-0.00473856681,-0.00801486174,-0.0121270876,-0.0086946861,0.00835037462,0.00136773587,0.031551998,-0.106844901,-0.00358314896,-0.00679883078,0.0283449104};
   stable_sort(choices.begin(),choices.end(),[](auto& a,auto& b){return a.gain>b.gain;});
   int n=0,attempted=0,keys[3],ds[3],bestN=res.op.size();double low=0,baseValue=fv.eval(res);
   State winner;
   for(auto q:choices){
    int k=q.k,d=q.d,p=base.pos[k];bool near=false;
    for(int j=0;j<n;j++)if(keys[j]==k&&abs(ds[j]-d)<3)near=true;
    if(near)continue;if(++attempted>24||!room())break;
    State desired=res;auto& a=desired.a;
    if(d<p)move_backward(a.begin()+d,a.begin()+p,a.begin()+p+1);else move(a.begin()+p+1,a.begin()+d+1,a.begin()+p);
    a[d]=k;if(display(desired)!=target)continue;
    Sched sc;sc.init(start,desired);State out;bool ok=false;
    for(int j=0;j<attempts&&!ok;j++)ok=sc.attempt(out,j,int(res.op.size())+1);
    if(!ok)continue;
    keys[n]=k;ds[n]=d;++n;
    int dir=d>p?1:-1;BitsCount pre=base.pre[p];double score=rerank[0]*(fv.eval(out)-baseValue);
    for(int j=p;j!=d;j+=dir){
     int y=out.a[j],t=min(j,j+dir);if(dir<0)pre.sub(cards[y].m);
     int dx,ex,dy,ey;U wx,wy;pre.query(cards[k].m,cards[k].end,dx,ex,wx);pre.query(cards[y].m,cards[y].end,dy,ey,wy);
     score+=dir*learnedCross(k,y,t,dx,dy,wx,wy,rerank);
     if(dir>0)pre.add(cards[y].m);
    }
    int m=out.op.size();if(m<bestN||(m==bestN&&score<low)){bestN=m;low=score;winner=std::move(out);}
    if(n==3)break;
   }
   if(n){if(!winner.op.empty())res=std::move(winner);return;}
  }
  for(auto q:opts){if(!room())break;
   State desired=res;int at=locate(desired,q.k);auto &a=desired.a;
   if(q.d<at)move_backward(a.begin()+q.d,a.begin()+at,a.begin()+at+1);else move(a.begin()+at+1,a.begin()+q.d+1,a.begin()+at);a[q.d]=q.k;
   if(display(desired)!=target)continue;
   Sched opt;opt.init(start,desired);State result;
   for(int r=0;r<attempts;r++)if(opt.attempt(result,r,int(res.op.size())+1)){res=std::move(result);return;}
  }
 }
} dpivot;
// v1237の固定集合・各紙ちょうど1回の判定。紙の形状にはprefixを仮定しない。
// 予算は初めて展開するDP状態だけを数え、打切りを不可の証明へ混ぜない。
namespace conflict_search {
#ifdef SEED_AUDIT
long seedChecks=0,seedUpdates=0;
#endif
using Bits=array<U,3>;
bool has(const Bits& m,int k){return m[k>>6]>>(k&63)&1;}
void mark(Bits& m,int k){m[k>>6]|=1ULL<<(k&63);}
bool contains(const Bits& wide,const Bits& small){return !((small[0]&~wide[0])|(small[1]&~wide[1])|(small[2]&~wide[2]));}
int count(const Bits& b){return pc(b[0])+pc(b[1])+pc(b[2]);}
Bits unite(const Bits& a,const Bits& b){return {a[0]|b[0],a[1]|b[1],a[2]|b[2]};}
struct Gate {Bits need;int size;};
struct Witnesses{array<Gate,6> opt;int n=0;};
struct Item{Bits mask;int merit,used,block,grip;U rem;int stuck=-1,obstructing=-1;};
// Assign a final gap to each moved sheet. Moving them in initial order is
// executable if each still owns a grip after its earlier peers are inserted.
struct GapWitness{
 static constexpr int C=32;
 const State& initial;const U goal;const Bits& selected;
 int n=0,nf=0,ids[C],gap[C],fixed[K],order[C],dest[C],usedN=0;
 U allowed[C],ink[C],wrong[C],badFixed[K],stationPrefix[K+1],tailAllow[C];
 int nodes=0,cut=0,found=0,bestLoss=INT_MAX;
 int stoppedAt=-1,stoppedPaper=-1,stoppedBy=-1,temporary=-1;U stoppedRem=~U(0);
 void recordConflict(int g,int mover,U coverage,int by){
  if(stoppedPaper<0||pc(~coverage)<pc(stoppedRem)){
   stoppedAt=g<nf?fixed[g]:-1;stoppedPaper=ids[mover];stoppedRem=~coverage;stoppedBy=ids[by];
  }
 }
 int incumbent[C]{},chosenOrder[C]{},chosenDest[C]{};
 GapWitness(const State& s,const Bits& m,U target):initial(s),goal(target),selected(m){
  U covered=0;stationPrefix[0]=0;
  for(int k:s.a){
   if(has(m,k)){ids[n]=k;gap[n]=nf;ink[n]=cards[k].m;wrong[n]=(cards[k].v^target)&cards[k].m;allowed[n]=ink[n]&~covered;n++;}
   else{fixed[nf]=k;badFixed[nf]=(cards[k].v^target)&cards[k].m;covered|=cards[k].m;stationPrefix[++nf]=covered;}
  }
 }
 bool dfs(int g,int level,U placed,U cov,const U* avail){
  while(g<nf&&!(badFixed[g]&~cov))cov|=cards[fixed[g++]].m;
  if(cov==~U(0)){
   int loss=0;
   for(int j=0;j<level;j++){
    int id=ids[order[j]],p=dest[j]+j;
    // An active sheet has to occupy this early gap. Compare that
    // commitment before choosing where to park still-inactive movers.
    loss+=2*max(0,incumbent[order[j]]-p)+max(0,shelfKey(id)-p);
   }
   if(loss<bestLoss){
    if(found)LOCAL_ONLY(trace_stats.count("better_active_witness"));
    bestLoss=loss;usedN=level;memcpy(tailAllow,avail,n*sizeof(U));
    copy_n(order,level,chosenOrder);copy_n(dest,level,chosenDest);
   }
   return ++found>=3;
  }
  if(++nodes>72)return false;
  for(int i=0;i<n;i++){
   if((placed>>i&1)||(wrong[i]&~cov)||!(ink[i]&~cov))continue;
   U next[C];memcpy(next,avail,n*sizeof(U));bool ok=true;
   for(int j=i+1;j<n;j++)if(gap[j]>g&&ids[i]!=temporary){next[j]&=~ink[i];if(!next[j]){recordConflict(g,j,cov,i);ok=false;break;}}
   if(!ok){++cut;continue;}
   order[level]=i;dest[level]=g;
   if(dfs(g,level+1,placed|(1ULL<<i),cov|ink[i],next))return true;
  }
  return false;
 }
 // Discard optical failures before the bounded physical search. A compatible
 // mover can only cover correctly coloured windows, so it is safe to use now.
 bool opticallyPossible()const{
  U rem=~U(0),left=lo(n);
  for(int g=0;rem&&g<=nf;g++){
   while(rem&&(g==nf||(rem&badFixed[g]))){
    int chosen=-1;
    for(U bits=left;bits;bits&=bits-1){
     int i=ctz(bits);if((ink[i]&rem)&&!(wrong[i]&rem)){chosen=i;break;}
    }
    if(chosen<0)return false;
    rem&=~ink[chosen];left^=U(1)<<chosen;
   }
   if(g<nf)rem&=~cards[fixed[g]].m;
  }
  return !rem;
 }
 bool build(State& out,const State& reference){
  for(int i=0;i<n;i++)if(!allowed[i])return false;
  int referencePos[K];for(int j=0;j<K;j++)referencePos[reference.a[j]]=j;
  for(int i=0;i<n;i++)incumbent[i]=referencePos[ids[i]];
  dfs(0,0,0,0,allowed);if(!found)return false;
  copy_n(chosenOrder,usedN,order);copy_n(chosenDest,usedN,dest);
  uint8_t layout[K];int len=0,q=0;bool used[C]{};
  for(int j=0;j<=nf;j++){
   while(q<usedN&&dest[q]==j){int i=order[q++];used[i]=true;layout[len++]=ids[i];}
   if(j<nf)layout[len++]=fixed[j];
  }
  Bits refMoved{};for(auto op:reference.op)mark(refMoved,op.k);
  int rank[K];for(int i=0;i<K;i++)rank[reference.a[i]]=i;
  // Inactive movers can land anywhere whose prefix masks their wrong ink.
  // Preserve every later original mover's last grip and approach the incumbent.
  for(int i=0;i<n;i++)if(!used[i]){
   int minGap=0;for(int j=i+1;j<n;j++)if(!(tailAllow[j]&~ink[i]))minGap=max(minGap,gap[j]);
   int at=-1,g=0,bestGap=-1,inv=0,best=INT_MAX;U front=0;
   for(int j=0;j<=len;j++){
    if(g>=minGap&&!(wrong[i]&~front)&&inv<best){at=j;best=inv;bestGap=g;}
    if(j<len){int y=layout[j];g+=!has(selected,y);front|=cards[y].m;// Prioritise proximity if the incumbent also moved the blocker.
     // A newly discovered blocker has no reference parking destination.
     const int refWeight=has(refMoved,ids[i])?64:24;
     inv+=refWeight*((rank[y]>rank[ids[i]])-(rank[y]<rank[ids[i]]))+pricing.price(ids[i],y);}
   }
   if(at<0)return false;
   for(int j=i+1;j<n;j++)if(gap[j]>bestGap)tailAllow[j]&=~ink[i];
   memmove(layout+at+1,layout+at,len-at);layout[at]=ids[i];len++;
  }
  for(int j=0;j<K;j++)rank[layout[j]]=j;
  Bits pending=selected;out=initial;out.op.clear();
  for(int i=0;i<n;i++){
   int k=ids[i],from=locate(out,k);int d=0;
   for(;d<K;d++){int y=out.a[d];if(!has(pending,y)&&rank[y]>rank[k])break;}
   moveCard(out,from,d-(d>from));pending[k>>6]&=~(1ULL<<(k&63));
  }
  return true;
 }
};

// The same gap certificate handles deletion before or after the broad search.
template<class Time>
void trim_set(const State& start,State& best,U target,const vector<State>& alternatives,const Time& room){
 if(best.op.empty()||!room(35e-6))return;
 Bits base{};vector<int> ids;
 for(auto mv:best.op)if(!has(base,mv.k)){ids.push_back(mv.k);mark(base,mv.k);}
 if(ids.size()>=GapWitness::C)return;
 int baseN=ids.size();
 auto test=[&](const Bits& m){
  if(count(m)>=int(best.op.size()))return;
  GapWitness cert(start,m,target);State path;
  if(cert.opticallyPossible()&&cert.build(path,best)&&path.op.size()<best.op.size()){
   LOCAL_ONLY(trace_stats.count("normalized_paths");trace_stats.count_by("normalized_saved",int(best.op.size()-path.op.size())));
   best=std::move(path);
  }
 };
 test(base);
 for(int i=0;i<baseN&&room(22e-6);i++){
  Bits m=base;m[ids[i]/64]^=U(1)<<(ids[i]%64);test(m);
 }
 // A different currently feasible optical witness can replace two moved papers.
 // Try its active newcomer against the incumbent movers absent from that route.
 if(baseN+1>=GapWitness::C||int(best.op.size())<=baseN-1)return;
 for(const State& alt:alternatives){
  if(alt.op.size()>best.op.size()+2||!room(25e-6))continue;
  Bits moved{};for(auto mv:alt.op)mark(moved,mv.k);
  vector<int> changeable;for(int k:ids)if(!has(moved,k))changeable.push_back(k);
  if(changeable.size()<2)continue;
  U cover=0;int newOwner=-1;
  for(int k:alt.a){
   if((cards[k].m&~cover)&&has(moved,k)&&!has(base,k)){newOwner=k;break;}
   cover|=cards[k].m;if(!~cover)break;
  }
  if(newOwner<0)continue;
  ids.push_back(newOwner);mark(base,newOwner);
  int limit=min<int>(6,changeable.size());
  for(int i=0;i<limit;i++)for(int j=0;j<i&&room(22e-6);j++){
   Bits m=base;m[changeable[i]/64]^=U(1)<<(changeable[i]%64);m[changeable[j]/64]^=U(1)<<(changeable[j]%64);test(m);
  }
  break;
 }
}

// Only the physical replay certifies this two-grip schedule. The relaxed
// witness supplies a target order and a sheet to delay, not a valid path.
bool temporarilyPark(const State& start,const Bits& selected,const State& goal,
                     int held,int obstructed,int far,int release,State& out){
 int rank[K];for(int i=0;i<K;i++)rank[goal.a[i]]=i;
 Bits pending=selected;
 out=start;out.op.clear();
 auto take=[&](int k,int temporaryAfter){
  int from=locate(out,k);U m=cards[k].m;
  for(int i=0;i<from&&m;i++)m&=~cards[out.a[i]].m;
  if(!m)return false;
  int dest=0;
  if(temporaryAfter>=0)dest=locate(out,temporaryAfter)+1;
  else{
   for(;dest<K;dest++){int y=out.a[dest];if(!has(pending,y)&&rank[y]>rank[k])break;}
   pending[k>>6]&=~(1ULL<<(k&63));
  }
  moveCard(out,from,dest-(dest>from));return true;
 };
 for(int k:start.a)if(has(pending,k)){
  if(!take(k,k==held?(far>=0?far:obstructed):-1))return false;
  if(k==release&&!take(held,-1))return false;
 }
 if(has(pending,held)&&!take(held,-1))return false;
 return out.a==goal.a;
}

struct Solver{
 Witnesses wit[K];array<U,K> bad{};Bits reference{};const State& initial;int bound;U target;double meritBest;int pos[K];
 struct Compatible {U rem=0;uint8_t ids[K];int n=0;};array<Compatible,128> table;
 const Compatible& compatible(U r){
  auto& c=table[(r*0x9e3779b97f4a7c15ULL)>>57];
  if(c.rem!=r){c.rem=r;c.n=0;for(int k:initial.a)if(!(bad[k]&r)&&(cards[k].m&r))c.ids[c.n++]=k;}
  return c;
 }
 struct Entry {Bits bits;unsigned generation;};
 unsigned generation=0;

 // The priority is an integer at most 15*(initial bound-1)+128.
 // Keep a linked list per present integer and recycle popped slots.
 struct Frontier{
  struct Link{Item item;int next;};
  vector<int> first,last;vector<Link> links;int smallest,free=-1;
  explicit Frontier(int limit):first(15*limit+129,-1),last(first),smallest(first.size()){links.reserve(256);}
  bool empty()const{return smallest==int(first.size());}
  const Item& top()const{return links[first[smallest]].item;}
  void pop(){
   int id=first[smallest];first[smallest]=links[id].next;
   if(first[smallest]<0){last[smallest]=-1;do{++smallest;}while(smallest<int(first.size())&&first[smallest]<0);}
   links[id].next=free;free=id;
  }
  void push(Item item){
   int q=item.merit,id;
   if(free>=0){id=free;free=links[id].next;links[id].item=item;}
   else{id=links.size();links.push_back({item,-1});}
   links[id].next=-1;
   if(last[q]>=0)links[last[q]].next=id;else first[q]=id;
   last[q]=id;
   smallest=min(smallest,q);
  }
 } queue;
 Solver(const State&s,const State& best,U tar):initial(s),bound(best.op.size()+1),target(tar),meritBest(best.op.size()+.75*fv.eval(best)),queue(best.op.size()){
  static unsigned nextGeneration=0;generation=++nextGeneration;
  for(auto mv:best.op)mark(reference,mv.k);
  int index=0;for(int k:s.a){pos[k]=index++;bad[k]=(target^cards[k].v)&cards[k].m;}
 }
 Bits truncatedGrips{};
 struct Head{Bits required{},fixed{};int cost;};
 vector<Head> head;Bits commonHead{};
 void add_head(Bits m,Bits b){
  if(count(m)>=bound)return;
  for(const auto& h:head)if(contains(m,h.required)&&contains(b,h.fixed))return;
  erase_if(head,[&](const Head& h){return contains(h.required,m)&&contains(h.fixed,b);});
  head.push_back({m,b,count(m)});
 }
 Bits wrongPrefix[N]{},correct[N]{};
 void collect_window_clauses(){
  U saw=0;
  for(int k:initial.a){U good=cards[k].m&~bad[k];saw|=good;
   for(U w=good;w;w&=w-1)mark(correct[ctz(w)],k);
   for(U w=bad[k]&~saw;w;w&=w-1)mark(wrongPrefix[ctz(w)],k);
  }
 }
 bool window_clauses(Head& h,const vector<Bits>* gates){
  for(int z=0;z<N;z++){
   const Bits& good=correct[z];const Bits& bad=wrongPrefix[z];
   if(contains(h.required,bad)||((good[0]&h.required[0])|(good[1]&h.required[1])|(good[2]&h.required[2])))continue;
   Bits shared{~U(0),~U(0),~U(0)};bool any=false;
   auto test=[&](const Bits& g){
    if((g[0]&h.fixed[0])||(g[1]&h.fixed[1])||(g[2]&h.fixed[2])||count(unite(h.required,g))>=bound)return;
    any=true;for(int j=0;j<3;j++)shared[j]&=g[j];
   };
   test(bad);
   for(int b=0;b<3&&!(any&&contains(h.required,shared));b++){
    for(U ks=good[b]&~h.fixed[b];ks&&!(any&&contains(h.required,shared));ks&=ks-1){
     int k=64*b+ctz(ks);for(const auto& g:gates[k])test(g);
    }
   }
   if(!any)return false;
   Bits before=h.required;h.required=unite(h.required,shared);h.cost=count(h.required);
   if(h.cost>=bound)return false;
   LOCAL_ONLY(trace_stats.count_by("window_forced",h.cost-count(before)));
  }
  return true;
 }

 bool tighten(Head& h){
  const int room=bound-1-h.cost;
  if(room<0)return false;
  U canForce=~U(0),canExtra=room?~U(0):0,hard=0;
  int depth[N]{};
  for(int k:initial.a){
   const bool forced=has(h.required,k);U m=cards[k].m;
   if(!(m&~hard&(forced?canForce:canExtra))){
    if(forced)return false;mark(h.fixed,k);
   }
   if(has(h.fixed,k)){hard|=m;continue;}
   if(!forced)for(U bits=m&canForce&~hard;bits;bits&=bits-1){
    int z=ctz(bits);if(++depth[z]>=room)canExtra&=~(1ULL<<z);
    if(depth[z]>room)canForce&=~(1ULL<<z);
   }
  }
  return true;
 }

 void make_heads(){
  collect_window_clauses();bool relevant[K]{};
  for(int f:initial.a)if(!bad[f]&&cards[f].m){
   relevant[f]=true;U left=~cards[f].m;
   for(int k:initial.a)if((cards[k].m&left)&&!(bad[k]&left))relevant[k]=true;
  }
  // A cap on the *cardinality* is safe. All nondominated initial windows
  // below that bound are kept, including witnesses omitted by the 6-way queue.
  static array<vector<Bits>,K> all;
  for(auto& g:all)g.clear();
  Bits above[N]{};int dep[N]{};U shallow=~U(0);
  for(int k:initial.a){
   for(U bits=cards[k].m&shallow;bits;bits&=bits-1){
    Bits m=above[ctz(bits)];mark(m,k);
    bool dominated=false;for(auto& g:all[k])if(contains(m,g)){dominated=true;break;}
    if(!dominated){erase_if(all[k],[&](const Bits&g){return contains(g,m);});all[k].push_back(m);}
   }
   auto& w=wit[k];
   if(all[k].size()>w.opt.size())mark(truncatedGrips,k);
   for(const Bits& g:all[k]){
    int n=count(g);if(w.n==int(w.opt.size())&&n>=w.opt.back().size)continue;
    int j=min(w.n,int(w.opt.size())-1);
    while(j&&w.opt[j-1].size>n){w.opt[j]=w.opt[j-1];--j;}
    w.opt[j]={g,n};w.n=min(w.n+1,int(w.opt.size()));
   }
   for(U bits=cards[k].m&shallow;bits;bits&=bits-1){int z=ctz(bits);mark(above[z],k);if(++dep[z]+1>=bound)shallow^=1ULL<<z;}
  }
  auto second=[&](int f,int start,Bits forced,Bits fixed){
   if(count(forced)>=bound)return;U left=~cards[f].m;if(!left){add_head(forced,fixed);return;}
   Bits skip=forced;
   for(int j=0;j<K;j++){
    int k=initial.a[j];U exposed=cards[k].m&left;if(!exposed)continue;
    if(!(exposed&bad[k])){
     if(j>=start&&!has(skip,k)){Bits fixed2=fixed;mark(fixed2,k);add_head(skip,fixed2);}
     for(auto& g:all[k]){
      if((g[0]&fixed[0])||(g[1]&fixed[1])||(g[2]&fixed[2]))continue;
      add_head(unite(forced,g),fixed);
     }
    }
    if(j>=start)mark(skip,k);
   }
  };
  Bits prefix{};
  for(int j=0;j<K;j++){
   int k=initial.a[j];if(!bad[k]&&cards[k].m){Bits fixed{};mark(fixed,k);second(k,j+1,prefix,fixed);for(auto& g:all[k])second(k,0,g,{});}
   if(cards[k].m)mark(prefix,k);
  }
  erase_if(head,[&](Head& h){
   int before=count(h.fixed);bool possible=tighten(h);
   int old=h.cost;
   if(possible)possible=window_clauses(h,all.data());
   if(possible&&h.cost!=old)possible=tighten(h);
   LOCAL_ONLY(trace_stats.count_by("root_bans",count(h.fixed)-before);if(!possible)trace_stats.count("root_impossible"));
   return !possible;
  });
  stable_sort(head.begin(),head.end(),[](const Head& x,const Head& y){return x.cost<y.cost;});
  if(!head.empty()){commonHead=head[0].required;for(const Head& h:head)for(int b=0;b<3;b++)commonHead[b]&=h.required[b];}

 }
 int wanted(const Bits&m,const Head& h)const{
  if((m[0]&h.fixed[0])||(m[1]&h.fixed[1])||(m[2]&h.fixed[2]))return K;
  return pc(h.required[0]&~m[0])+pc(h.required[1]&~m[1])+pc(h.required[2]&~m[2]);
 }
 void rooted(const Bits& m,State& best){
  int nearest=K;for(const auto& h:head)nearest=min(nearest,wanted(m,h));
  if(nearest>=K)return;int n=0;for(auto& h:head)if(wanted(m,h)==nearest){
   insert(unite(m,h.required),best);if(!nearest||++n>=3)break;
  }
 }

 void substitute(const Item& node,State& best){
  int old=node.obstructing;if(old<0||has(commonHead,old))return;
  U wantedInk=node.rem&cards[old].m;
  if(node.block>=0&&(wantedInk&bad[node.block]))wantedInk&=bad[node.block];
  if(!wantedInk)return;
  struct Alternative{int key=INT_MAX,active=-1;Bits mask{};};array<Alternative,2> choice{};
  Bits coreSet=node.mask;coreSet[old>>6]&=~(U(1)<<(old&63));
  const auto& can=compatible(node.rem);
  for(int j=0;j<can.n;j++){
   int k=can.ids[j];if(has(node.mask,k)||!(wantedInk&cards[k].m))continue;
   const auto& w=wit[k];
   for(int g=0;g<w.n;g++){
    if(has(w.opt[g].need,old))continue;
    Bits m=unite(coreSet,w.opt[g].need);int cost=count(m);
    if(cost>=bound||m==choice[0].mask||m==choice[1].mask)continue;
    int key=16*cost+3*pc(wantedInk&~cards[k].m)+min(7,pc(node.rem&~cards[k].m));
    Alternative next{key,k,m};
    if(key<choice[0].key){choice[1]=choice[0];choice[0]=next;}
    else if(key<choice[1].key)choice[1]=next;
   }
  }
  for(auto& q:choice)if(q.active>=0){
   Bits reached{};U fixed=0;
   for(int y:initial.a){
    if(has(q.mask,y)&&(cards[y].m&~fixed))mark(reached,y);else fixed|=cards[y].m;
    if(!~fixed)break;
   }
   if(!has(reached,q.active))continue;
   int prior=best.op.size();unsigned seenBefore=generated;insert(reached,best,true);
   LOCAL_ONLY(trace_stats.count_by("new_substitute_sets",generated-seenBefore);trace_stats.count_by("substitution_gains",prior-int(best.op.size())));
  }
 }
 // Only the remaining windows decide which sheets are color-compatible.
 // Once compatible, every printed cell stays correct as more cells are covered.
 const Bits& compatibleMask(U remaining)const{
  struct Entry{U product=0,remaining=0;Bits match{};};
  static array<Entry,4096> cache{};
  auto& e=cache[((remaining^(target>>21))*0x9e3779b97f4a7c15ULL)>>52];
  if(e.product!=target||e.remaining!=remaining){
   e.product=target;e.remaining=remaining;e.match={};
   for(int k=0;k<K;k++)if(!(bad[k]&remaining))mark(e.match,k);
  }
  return e.match;
 }
 int firstStationary(const Bits& moved,U rem)const{
  struct Entry{U rem;unsigned generation;uint8_t first[K],n;};
  static array<Entry,2048> table{};
  auto& e=table[(rem*0x9e3779b97f4a7c15ULL)>>53];
  if(e.generation!=generation||e.rem!=rem){
   e.generation=generation;e.rem=rem;e.n=0;
   for(int k:initial.a)if(cards[k].m&rem){
    e.first[e.n++]=k;if(e.n>=bound)break;
   }
  }
  for(int i=0;i<e.n;i++)if(!has(moved,e.first[i]))return e.first[i];
  return -1;
 }
 pair<U,int> optical(const Bits& m,U rem=~U(0))const{
  while(rem){
   // Previously consumed stationary sheets have no ink on the open windows.
   // The first remaining one must therefore occur among the first |m|+1 hits.
   int k=firstStationary(m,rem);
   if(k>=0&&!(bad[k]&rem)){rem&=~cards[k].m;continue;}
   const auto& match=compatibleMask(rem);U ink=0;
   for(int b=0;b<3;b++)for(U q=m[b]&match[b];q;q&=q-1)ink|=cards[64*b+ctz(q)].m;
   if(!(ink&rem))return {rem,k};
   rem&=~ink;
  }
  return {rem,-1};
 }
 unsigned generated=0,expanded=0,solved=0,regrips=0;
 void insert(Bits m,State& best,bool reopened=false,const Item* parent=nullptr,const Bits* gate=nullptr){
  int paid=count(m);if(paid>=bound)return;
  U key=(m[0]*0x9e3779b97f4a7c15ULL)^(m[1]*0xbf58476d1ce4e5b9ULL)^(m[2]*0x94d049bb133111ebULL);
  // An order generates at most a few thousand sets. Reusing the table
  // across orders avoids zeroing it and keeps already-explored collisions.
  static array<Entry,16384> seen{};
  unsigned origin=(key^(key>>32))&(seen.size()-1),slot=origin;
  for(int probe=0;probe<24;probe++){
   auto& e=seen[slot];if(e.generation!=generation)break;
   if(e.bits==m)return;
   slot=(slot+1)&(seen.size()-1);
  }
  seen[slot]={m,generation};generated++;
  int lower=0,grip=-1;
  // A complete original grip stays complete in every superset of its movers.
  // Roots, substitutions and children with an unresolved parent are checked in full.
  Bits check=m;
  if(parent&&parent->grip<0)for(int b=0;b<3;b++)check[b]&=~parent->mask[b];
  // Each blocker in a complete window prefix can itself be grasped
  // through that window. Only truncated witness lists may omit this proof.
  if(gate)for(int b=0;b<3;b++)check[b]&=~(*gate)[b]|truncatedGrips[b];
  for(int b=0;b<3;b++)for(U q=check[b];q;q&=q-1){
   int k=64*b+ctz(q),least=K;auto&w=wit[k];
   for(int j=0;j<w.n;j++){
    auto&g=w.opt[j].need;int lacking=pc(g[0]&~m[0])+pc(g[1]&~m[1])+pc(g[2]&~m[2]);
    least=min(least,lacking);if(!least)break;
   }
   if(least+paid>=bound)return;
   if(least>lower){lower=least;grip=k;}
  }
  int prefixNeed=K;for(auto& h:head){prefixNeed=min(prefixNeed,wanted(m,h));if(!prefixNeed)break;}
  if(prefixNeed+paid>=bound){LOCAL_ONLY(trace_stats.count("root_pruned"));return;}
  lower=max(lower,prefixNeed);
  auto [rem,blocked]=optical(m,parent&&parent->stuck<0&&parent->block>=0?parent->rem:~U(0));
  if(!rem&&grip<0){
    if(paid<GapWitness::C){
     GapWitness cert(initial,m,target);State path;
     if(cert.build(path,best)){
      if(path.op.size()>=unsigned(bound))return;
      LOCAL_ONLY(trace_stats.count("grip_witness_paths");trace_stats.count_by("grip_witness_gain",bound-int(path.op.size())));
      double merit=path.op.size()+.75*fv.eval(path);
      if(merit<meritBest-1e-8){
       LOCAL_ONLY(if(best.op.size()==path.op.size())trace_stats.count("equal_selected"));
       LOCAL_ONLY(if(reopened){trace_stats.count("grip_reopened_paths");trace_stats.count_by("grip_reopened_saved",int(best.op.size()-path.op.size()));});
       meritBest=merit;bound=path.op.size()+1;best=std::move(path);solved++;
      }
     }
     else if(paid+1<bound&&cert.stoppedPaper>=0){
      if(regrips<9&&paid+1<int(best.op.size())){
       ++regrips;GapWitness flex(initial,m,target);flex.temporary=cert.stoppedBy;
       State desired;
       if(flex.build(desired,best)){
        int last=cert.stoppedPaper;
        for(int i=0;i<flex.n;i++)if(pos[flex.ids[i]]>pos[last]&&!(flex.allowed[i]&~cards[cert.stoppedBy].m))last=flex.ids[i];
        for(int q=0;q<4;q++){
         int after=(q%2)?last:cert.stoppedPaper,release=q<2?after:-1;
         if(q%2&&last==cert.stoppedPaper)continue;
         State two;if(!temporarilyPark(initial,m,desired,cert.stoppedBy,cert.stoppedPaper,after,release,two))continue;
         if(!validPlan(initial,two,target))abort();
         LOCAL_ONLY(trace_stats.count("two_grip_paths");trace_stats.count_by("two_grip_saved",int(best.op.size())-int(two.op.size())));
         double score=two.op.size()+.75*fv.eval(two);
         if(two.op.size()<best.op.size()&&score<meritBest){meritBest=score;bound=two.op.size()+1;best=std::move(two);solved++;}
         break;
        }
       }
       LOCAL_ONLY(trace_stats.count("two_grip_queries"));
      }
      // A physical witness can improve after adding sheets even when the
      // relaxed covering process already finished. Reopen its tightest gap.
      int outside=paid-(pc(m[0]&reference[0])+pc(m[1]&reference[1])+pc(m[2]&reference[2]));
      int q=(paid+1)*12+min(32,pc(cert.stoppedRem))*4+outside;
      queue.push({m,q,paid,cert.stoppedAt,-1,cert.stoppedRem,cert.stoppedPaper,cert.stoppedBy});
      LOCAL_ONLY(trace_stats.count("grip_reopened_nodes"));
     }
     LOCAL_ONLY(trace_stats.count("grip_witness_queries");trace_stats.count_by("grip_witness_prunes",cert.cut));
    }
    return;
  }
  int outside=paid-(pc(m[0]&reference[0])+pc(m[1]&reference[1])+pc(m[2]&reference[2]));
  int priority=(paid+lower)*12 + min(32,pc(rem))*4 + outside*(reopened?1:3);
  queue.push({m,priority,paid,blocked,grip,rem});
 }
 template<class Time> void run(State&best,const Time&room,const vector<State>& paths){
  constexpr int nodes=2000,children=14000;
  make_heads();for(auto& h:head)insert(h.required,best);
  LOCAL_ONLY(trace_stats.count_by("two_roots",head.size()));
  // All valid proposals describe a moved-sheet set. Give a small diverse
  // collection to the same optical closure and physical scheduling search.
  vector<pair<Bits,int>> alternatives;
  for(const auto& path:paths){
   if(path.op.size()>best.op.size()+2)continue;
   Bits m{};for(auto op:path.op)mark(m,op.k);
   if(m==reference||any_of(alternatives.begin(),alternatives.end(),[&](auto& p){return p.first==m;}))continue;
   alternatives.emplace_back(m,path.op.size());
  }
  vector<Bits> seeds{reference};
  while(seeds.size()<4&&!alternatives.empty()){
   auto diversity=[&](const auto& p){
    int dist=16;
    for(const auto& seed:seeds){
     Bits d{p.first[0]^seed[0],p.first[1]^seed[1],p.first[2]^seed[2]};
     dist=min(dist,count(d));
    }
    return dist-2*p.second;
   };
   auto choice=alternatives.begin();int highest=diversity(*choice);
   for(auto it=next(choice);it!=alternatives.end();++it)if(int v=diversity(*it);v>highest){highest=v;choice=it;}
   seeds.push_back(choice->first);alternatives.erase(choice);
  }
  for(size_t seedIndex=0;seedIndex<seeds.size()&&room(35e-6);seedIndex++){
   Bits source=seeds[seedIndex];
   int seq[K],nm=0;for(int k:initial.a)if(has(source,k))seq[nm++]=k;
   int initialSpan=seedIndex?0:6;
   unsigned before=solved;
   if(seedIndex)rooted(source,best);
   for(int i=0;i<nm&&room(35e-6);i++){
    for(int j=max(0,i-initialSpan);j<=i;j++){
     Bits seed=source;seed[seq[i]>>6]&=~(1ULL<<(seq[i]&63));seed[seq[j]>>6]&=~(1ULL<<(seq[j]&63));
     for(int t=0;t<nm;t++){
      int k=seq[t];if(!has(seed,k))continue;
      auto &w=wit[k];bool ok=false;for(int q=0;q<w.n;q++)if(contains(seed,w.opt[q].need)){ok=true;break;}
      if(!ok)seed[k>>6]&=~(1ULL<<(k&63));
     }
     rooted(seed,best);
    }
   }
   if(seedIndex){
    LOCAL_ONLY(trace_stats.count("alternative_sets");trace_stats.count_by("alternative_updates",solved-before));
#ifdef SEED_AUDIT
    ++seedChecks;seedUpdates+=solved-before;
#endif
   }
  }
  while(!queue.empty()&&expanded<unsigned(nodes)&&generated<unsigned(children)&&room(18e-6)){
   auto node=queue.top();queue.pop();if(node.used+1>=bound)continue;expanded++;
   if(node.stuck>=0){
    for(int j=0;j<wit[node.stuck].n;j++){
     Bits extra=unite(node.mask,wit[node.stuck].opt[j].need);
     if(extra!=node.mask)insert(extra,best,true,&node,&wit[node.stuck].opt[j].need);
    }
    if(node.block>=0){auto skip=node.mask;mark(skip,node.block);insert(skip,best,true,&node);}
    substitute(node,best);
   }
   if(node.grip>=0&&(!node.rem||pc(node.rem)<9)){
    auto&w=wit[node.grip];for(int j=0;j<w.n;j++)insert(unite(node.mask,w.opt[j].need),best,false,&node,&w.opt[j].need);
    continue;
   }
   const auto& can=compatible(node.rem);int blockedPos=node.block<0?K:pos[node.block],stop=K;
   for(int j=0;j<can.n;j++){
    int k=can.ids[j];if(has(node.mask,k))continue;
    // A moved display sheet must be graspable through at least one initial
    // window. Add its required blockers at the same time, merging equal unions.
    const auto& w=wit[k]; Bits added[6];int addedN=0;
    for(int g=0;g<w.n;g++){
     Bits take=unite(node.mask,w.opt[g].need);
     if(count(take)>=bound)continue;
     bool duplicate=false;for(int h=0;h<addedN;h++)if(added[h]==take){duplicate=true;break;}
     if(duplicate)continue;
     added[addedN++]=take;insert(take,best,node.stuck>=0,&node,&w.opt[g].need);
     if(count(take)==node.used+1)break;
    }
    LOCAL_ONLY(trace_stats.count_by("direct_grip_branches",addedN));
    if(pos[k]>=blockedPos&&stop==K)stop=pos[k];
   }
   if(blockedPos<stop){Bits drop=node.mask;for(int i=blockedPos;i<stop;i++)if(cards[initial.a[i]].m&node.rem)mark(drop,initial.a[i]);insert(drop,best,false,&node);}
  }
 }
};
template<class Time>void improve(const State& cur,State&best,U goal,const Time&room,const vector<State>& paths){
 if(best.op.size()<4||!room(120e-6))return;
 Solver engine(cur,best,goal);int old=best.op.size();engine.run(best,room,paths);
 LOCAL_ONLY(trace_stats.count_by("conflict_nodes",engine.expanded);trace_stats.count_by("conflict_children",engine.generated);trace_stats.count_by("conflict_paths",engine.solved);trace_stats.count_by("conflict_saved",old-int(best.op.size())));
}
}


int main(){
 constexpr double cpuBudget=1.82, searchCap=.00450;
 TimeKeeper time_keeper(cpuBudget);
 const auto wallStart=chrono::steady_clock::now();
 constexpr int priorMove=7,preCount=1;
 LOCAL_ONLY(for(const char* key:{"design_attempts","design_feasible","design_uncertified","design_failed","design_selected","graph_evaluated","graph_accepted","graph_retained","graph_certification_reverts","orders","T","universal_orders","backupDesign_calls","rescue_calls","resetDisplay_calls"})trace_stats.count_by(key,0));
 ios::sync_with_stdio(false);cin.tie(nullptr);int n,k,p,r;
 if(!time_keeper.wait_judge([&](){return bool(cin>>n>>k>>p>>r);}))return 0;
 for(int a=0;a<P;a++){string s;time_keeper.wait_judge([&](){cin>>s;});for(int z=0;z<N;z++)if(s[z]=='1')A[a]|=1ULL<<z;}
 // 注文ごとの予算配分は原文のまま、main開始からのCPU時間で判定する。
 auto elapsed=[&](){return time_keeper.exact_elapsed_sec();};
 LOCAL_TIME(trace_stats,"selectDesign",[&](){selectDesign(time_keeper);});
 if(!universal){frozenGeometry=true;findHints();}
 makeRescues();
 auto wall=[&](){return chrono::duration<double>(chrono::steady_clock::now()-wallStart).count();};
 double designWall=wall();double designCPU=elapsed();State cur;array<int,K> rank;
 for(int i=0;i<K;i++)cur.a[i]=i;
 stable_sort(cur.a.begin(),cur.a.end(),[](int i,int j){return home[i]<home[j];});
 if(!universal){
  if(haveLearnedShelf){cur.a=learnedShelf;for(int i=0;i<K;i++)shelfRank[cur.a[i]]=i;shelfReady=true;}
  else arrangeShelf(cur);
  fv.init(cur);pricing.init();
 }
 for(int i=0;i<K;i++){int id=cur.a[i];rank[id]=i;string s(N,'.');for(int z=0;z<N;z++)if(cards[id].m>>z&1)s[bitMap[z]]='0'+(cards[id].v>>z&1);cout<<s<<'\n';}cout.flush();

 liveTailCache=true;
 const auto initialRank=shelfRank;
 double carryBudget=0,meanDifficulty=0;
 // Measure a full order from one input boundary to the next. The work inside
 // a fast order still respects the controller's node, child and route limits.
 double meanFullCPU=.0011,meanFullWall=.0012,lastCPU=0,lastWall=0;
 int fullSamples=0;bool lastFull=false;
#ifdef FULL_ORDER_AUDIT
 int fastOrders=0,interrupted=0;
#endif
 for(int t=0;t<r;t++){
  int a;if(!time_keeper.wait_judge([&](){return bool(cin>>a);})||a<0)return 0;if(!universal)pricing.update(cur,t);cur.op.clear();cur.front=0;cur.cover=0;queryStamp++;
  if(universal){State start=cur;if(!taildp.solve(cur,A[a])||!validPlan(start,cur,A[a])){cur=std::move(start);resetDisplay(cur,a);}LOCAL_ONLY(trace_stats.count("orders");trace_stats.count_by("T",int64_t(cur.op.size()));trace_stats.count("universal_orders"));cout<<cur.op.size()<<'\n';for(auto m:cur.op)cout<<int(rank[m.k])<<' '<<int(m.d)<<'\n';cout.flush();continue;}
  const double unitBudget=(cpuBudget-designCPU)/r;
  double endAt=designCPU+unitBudget*(t+1)+.0002;
  // Spend only solver process CPU against the per-order allowance.
  // Let early orders use their ordinary CPU allocation. If the program is
  // running behind later in the interaction, protect time for the mandatory
  // plan and the remaining judge round trips.
  const double fraction=double(t+1)/r;
  const double wallEnd=max(1.20,designWall+(1.88-designWall)*fraction+.04*(1-fraction));
  const double beginCPU=elapsed(),beginWall=wall();
  if(lastFull){
   ++fullSamples;double rate=1./min(72,fullSamples);
   meanFullCPU+=rate*(beginCPU-lastCPU-meanFullCPU);
   meanFullWall+=rate*(beginWall-lastWall-meanFullWall);
  }
  lastCPU=beginCPU;lastWall=beginWall;
  const int left=r-t;
  double availableCPU=(cpuBudget-.009-beginCPU)/left,availableWall=(1.88-.012-beginWall)/left;
  double projectedCPU=designCPU+(cpuBudget-.01-designCPU)*double(t)/r;
  double projectedWall=designWall+(1.88-.012-designWall)*double(t)/r;
  bool fast=(fullSamples<8 || (availableCPU>1.02*meanFullCPU && availableWall>1.02*meanFullWall)
             || (beginCPU+.006<projectedCPU && beginWall+.008<projectedWall))
            && beginCPU<cpuBudget-.007 && beginWall<1.88-.010;
  lastFull=fast;
#ifdef FULL_ORDER_AUDIT
   fastOrders+=fast;
#endif
  // Large local searches can exhaust a case's slack. Refresh the expensive
  // process/wall clocks only once per 64 work checks while running uncapped.
  const double fenceCPU=min(cpuBudget-.001,beginCPU+.007);
  const double fenceWall=min(1.88,beginWall+.013);
  unsigned ticks=0;
  auto fullRoom=[&](double reserve){
   if(fast&&!(++ticks&63U)&&(elapsed()+reserve>=fenceCPU||wall()+reserve>=fenceWall)){
    fast=false;lastFull=false;
#ifdef FULL_ORDER_AUDIT
    ++interrupted;
#endif
   }
   return fast;
  };
  auto room=[&](double reserve=0.){return fullRoom(reserve)||(elapsed()+reserve<endAt&&wall()+reserve<wallEnd);};
  State best,bestPrep=cur;int bestcost=10000,btype=-1;
  vector<State> alternatives;alternatives.reserve(16);
  auto record=[&](const State& c){
    if(c.op.size()>size_t(bestcost+2)||alternatives.size()>=40)return;
    for(auto&s:alternatives)if(s.a==c.a)return;
    alternatives.push_back(c);
  };
  struct Tested {array<uint8_t,K> stack;U cover;unsigned short cost;uint8_t front;};
  array<Tested,48> tested;int testedN=0;
  auto solved=[&](State candidate,const State&base,int type){
     if(candidate.op.size()<=unsigned(bestcost+1)){
      Sched sc;sc.run(cur,candidate,8);
     }
     record(candidate);
     if((int)candidate.op.size()<bestcost){best=std::move(candidate);bestcost=best.op.size();btype=type;bestPrep=base;}
  };
  auto test=[&](State const&base,int type){
   if(base.op.size()>unsigned(bestcost+1))return;
   for(int i=0;i<testedN;i++){
     auto& prev=tested[i];
     if(prev.cover==base.cover && prev.front==base.front && prev.cost<=base.op.size() &&
        !memcmp(prev.stack.data(),base.a.data(),K))return;
   }
   if(testedN<int(tested.size()))tested[testedN++]={base.a,base.cover,(unsigned short)base.op.size(),(uint8_t)base.front};
   State candidate=base;
   if(taildp.solve(candidate,A[a],false,nullptr,bestcost-int(base.op.size())+6))
    solved(std::move(candidate),base,type);
  };
  if(room(110e-6))test(cur,0);
  State s=cur,other=cur;
  bool aok=fetchCore(s,core[a],false),bok=false;
  if(core[a].size()>1)bok=fetchCore(other,core[a],true);
  if(aok)test(s,1);
  if(bok)test(other,2);
  if(core[a].size()>1&&room(110e-6)){
   State partial=cur;vector<int> first={core[a][0]};if(fetchCore(partial,first,false))test(partial,3);
  }
  if(room(110e-6)){
   for(int j=0;j<K;j++)if(j!=core[a][0]&&!((cards[j].v^A[a])&cards[j].m)){
    State partial=cur;vector<int> first={j};if(fetchCore(partial,first,false))test(partial,4);
   }
  }
  if(core[a].size()>1&&room(110e-6)){
   int first=core[a][0];U rem=~cards[first].m;
   vector<pair<int,int>> good;
   for(int j=0;j<K;j++)if(j!=first&&j!=core[a][1]&&!(rem&cards[j].m&(cards[j].v^A[a]))&&pc(rem&low&~cards[j].m)<=2)good.emplace_back(-pc(rem&cards[j].m),j);
   sort(good.begin(),good.end());if(good.size()>6)good.resize(6);
   for(auto [priority,id]:good){vector<int> seq={first,id};for(int q=0;q<2;q++){State p=cur;if(fetchCore(p,seq,q))test(p,5);}}
  }
  if(!hints[a].empty()&&room(110e-6)){
   vector<pair<int,State>> choices;
   for(auto&h:hints[a])for(int z=0;z<(h.n<2?1:2);z++){State p=cur;if(gripRoute(p,h,z)&&int(p.op.size())<bestcost){
     int estimate=3*p.op.size()+h.est-h.rank;choices.emplace_back(estimate,std::move(p));}}
   sort(choices.begin(),choices.end(),[](auto&a,auto&b){return a.first<b.first;});
   struct Ready{int at;State result;};vector<Ready> feasible;
   for(int z=0;z<min<int>(8,choices.size());z++){
      const State& p=choices[z].second;bool dup=false;
      for(auto& old:feasible){const auto& q=choices[old.at].second;if(p.front==q.front&&p.cover==q.cover&&p.a==q.a){dup=true;break;}}
      if(dup)continue;State done=p;if(taildp.solve(done,A[a]))feasible.push_back({z,std::move(done)});
   }
   stable_sort(feasible.begin(),feasible.end(),[](auto& l,auto& r){return l.result.op.size()<r.result.op.size();});
   for(int z=0;z<min<int>(4,feasible.size());z++){auto& val=feasible[z];solved(std::move(val.result),choices[val.at].second,6);}
  }
  if(bestcost>=10000){
   State more=aok?s:(bok?other:cur);
   for(int tr=0;tr<64&&display(more)!=A[a];tr++){
    auto cs=candidates(more,A[a]);if(cs.empty())break;
    auto x=cs[0];if(!fetch(more,x.card,more.front,1ULL<<x.z,true))break;
    test(more,6);if(bestcost<10000)break;
   }
   if(bestcost>=10000){if(display(more)!=A[a]){more=cur;rescue(more,a);}best=std::move(more);bestcost=best.op.size();}
  }
  // 主紙をまだ動かさず、主紙から見える窓だけで合法な尾部操作を計画する。
  // 最浅取得窓を塞ぐ紙は主紙の取得にも必要なので、その紙の退避を先行させる。
  // 初期計画で残った難しさに応じて、後段の探索時間を注文間で配分する。
  // 終盤は累積補正を0に戻し、ケース全体のCPU予算を守る。
  const double difficulty=double(bestcost)-.8*min<int>(s.op.size(),bok?other.op.size():s.op.size());
  if(t==0)meanDifficulty=difficulty;
  meanDifficulty+=(difficulty-meanDifficulty)/min(64,t+1);
  const double weight=clamp((difficulty-meanDifficulty)/5.,-1.,1.);
  carryBudget=clamp(.90*carryBudget+.8*weight*unitBudget,-4*unitBudget,4*unitBudget);
  endAt+=carryBudget*min(1.,double(r-1-t)/8);
  if(room(120e-6)){
   auto buildBefore=[&](const vector<int>& ids, const State& ordinary, int policy){
    U cv=0;for(int id:ids)cv|=cards[id].m;if(!~cv)return;
    State plan=cur;plan.front=0;plan.cover=cv;
    bool cheap[K]{};for(auto move:ordinary.op)cheap[move.k]=true;for(int k:ids)cheap[k]=false;
    bool ok=policy?taildp.solve<true>(plan,A[a],true,nullptr,bestcost+3,cheap):
                   taildp.solve(plan,A[a],true,nullptr,bestcost+3);
    if(!ok||plan.op.empty())return;
    struct Counter{
     U b[8]{};
     void add(U m){for(int j=0;m;j++){U c=m&b[j];b[j]^=m;m=c;}}
     void sub(U m){for(int j=0;m;j++){U c=m&~b[j];b[j]^=m;m=c;}}
     int depth(U m)const{
      if(!m)return 999;
      int d=0;for(int j=7;j>=0;j--){U a=m&~b[j];if(a)m=a;else d+=1<<j;}
      return d;
     }
    };
    int n=plan.op.size(),nc=ids.size(),pos[2]={};Counter hit[2]{};
    for(int h=0;h<nc;h++){
     pos[h]=locate(cur,ids[h]);for(int j=0;j<pos[h];j++)hit[h].add(cards[cur.a[j]].m);
    }
    struct Choice{int rank,end,reverse;};vector<Choice> opts;
    State prefix=cur;
    for(int i=0;i<min(n,18);i++){
     auto mv=plan.op[i];int from=locate(prefix,mv.k);
     for(int h=0;h<nc;h++){
      int k=ids[h];bool above=from<pos[h];pos[h]-=above;bool newAbove=mv.d<=pos[h];pos[h]+=newAbove;
      if(newAbove&&!above)hit[h].add(cards[mv.k].m);
      if(above&&!newAbove)hit[h].sub(cards[mv.k].m);
     }
     moveCard(prefix,from,mv.d);
     if(i>0&&i+1<n&&!cheap[mv.k]&&i+1!=n/2)continue;
     int wrong=pc((display(prefix)^A[a])&~cv);
     for(int rev=0;rev<nc;rev++){
      int cost=hit[rev].depth(cards[ids[rev]].m);
      if(nc==2)cost+=hit[rev^1].depth(cards[ids[rev^1]].m&~cards[ids[rev]].m);
      opts.push_back({cost*8+(i+1)*priorMove+(n-i-1)*3+wrong,i+1,rev});
     }
    }
    stable_sort(opts.begin(),opts.end(),[](auto x,auto y){return x.rank<y.rank;});
    for(int j=0;j<min<int>(preCount,opts.size())&&room(60e-6);j++){
     const auto c=opts[j];State p=cur;
     for(int i=0;i<c.end;i++){auto mv=plan.op[i];moveCard(p,locate(p,mv.k),mv.d);}
     if(fetchCore(p,ids,c.reverse)){
      int old=bestcost;test(p,13);
      LOCAL_ONLY(trace_stats.count("pre_tail_trials");if(bestcost<old)trace_stats.count_by("pre_tail_saved",old-bestcost));
     }
    }
   };
   if(aok)buildBefore(core[a],s,true);
  }
  if(room(110e-6)){
   auto base=bestPrep;for(int q=0;q<2;q++){State out;if(multiHole(base,best,A[a],out,bestcost,q,2)){best=std::move(out);bestcost=best.op.size();btype=11;}}
  }
  if(room(110e-6)){
   State ref=best;vector<vector<int>>seqs={core[a]};
   if(bestPrep.front>0&&bestPrep.front<=3){vector<int>v(bestPrep.a.begin(),bestPrep.a.begin()+bestPrep.front);if(v!=core[a])seqs.push_back(v);}
   for(auto&seq:seqs)for(int t=0;t<(seq.size()==1?1:2);t++){State v=cur;if(guidedPrep(v,seq,ref,t,true))test(v,9);}
  }
  if(room(110e-6)){State out;if(ghost(cur,core[a],A[a],out,bestcost,12))best=std::move(out),bestcost=best.op.size(),btype=7;}

  if(room(110e-6)){Trimmer pr;pr.run(cur,best,A[a]);}

  if(room(110e-6))conflict_search::trim_set(cur,best,A[a],alternatives,room);
  const double terminalReserve=clamp(.23*unitBudget,45e-6,260e-6);
  const double searchEnd=min(endAt-terminalReserve,elapsed()+searchCap);
  unsigned searchTicks=0;bool searchAllowed=true;
  auto searchRoom=[&](double reserve=0.){
   if(!(searchTicks++&15U))searchAllowed=elapsed()+reserve<searchEnd&&wall()+reserve<wallEnd-terminalReserve;
   return searchAllowed;
  };
  conflict_search::improve(cur,best,A[a],searchRoom,alternatives);
  if(!alternatives.empty()&&room()){
   const double weight=0.75*min(1.0,(r-1-t)/4.0);
   double bestVal=best.op.size()+weight*fv.eval(best);int found=-1,paid=0;
   for(int i=0;i<int(alternatives.size());i++){
    auto &c=alternatives[i];if(c.op.size()>best.op.size()+2||c.a==best.a)continue;
    if(paid++>=15||!room())break;
    double q=c.op.size()+weight*fv.eval(c);
    if(q<bestVal){bestVal=q;found=i;}
   }
   if(found>=0){
    auto c=std::move(alternatives[found]);if(room()){Trimmer tr;tr.run(cur,c,A[a]);}
    if(c.op.size()<=384)best=std::move(c);
   }
  }
  dpivot.improve(cur,best,A[a],a,room);
  if(!validPlan(cur,best,A[a])){best=cur;resetDisplay(best,a);}
  conflict_search::trim_set(cur,best,A[a],alternatives,room);
  cur=std::move(best);pricing.observe(cur,t);
  if(runningRequests){
   runningRequests->observe(cur,a);
   // Only completed orders contribute to the new restoration prices.
   if(t+1<r&&(t+1)%256==0){
    runningRequests->arrange();const auto& desired=runningRequests->order;
    array<int,K> next{};for(int i=0;i<K;i++)next[desired[i]]=i;
    // The terminal model was trained against the initial reference shelf.
    // Change restoration prices while retaining that model's coordinates.
    for(int k=0;k<K;k++)for(int y=k+1;y<K;y++){
     int change=int(next[k]<next[y])-int(shelfRank[k]<shelfRank[y]);
     pricing.change[k][y]+=64*change;pricing.change[y][k]=-pricing.change[k][y];
    }
    for(auto& q:pricing.neighbors){
     int old=initialRank[q.k]<initialRank[q.y]?1:-1,newer=next[q.k]<next[q.y]?1:-1;
     // Leave conflicting directions to the independently learned grip histories.
     q.orientation=(old+newer)/2;
    }
    shelfRank=next;
   }
  }
  if(!cur.op.empty()&&display(cur)!=A[a])abort();
  LOCAL_ONLY(trace_stats.count("orders");trace_stats.count_by("T",int64_t(cur.op.size())));
  cout<<cur.op.size()<<'\n';for(auto m:cur.op)cout<<rank[m.k]<<' '<<int(m.d)<<'\n';cout.flush();
 }
#ifdef SEED_AUDIT
 cerr<<"SEEDS "<<conflict_search::seedChecks<<" "<<conflict_search::seedUpdates<<"\n";
#endif
#ifdef FULL_ORDER_AUDIT
 cerr<<"full_orders "<<fastOrders<<" emergency "<<interrupted<<" mean "<<meanFullCPU<<" "<<meanFullWall<<'\n';
#endif
 LOCAL_ONLY(trace_stats.summary());
}
