# B-101 C++忠実再移植の静的監査

比較対象は、提供C++ `pasted-text.txt`、その旧Rust移植 `v081_deep_terminal_hybrid.rs`、再移植 `v086_cpp_faithful_runtime.rs` である。評価前にはsolverを起動せず、ソース比較とlocal/提出両featureのrelease buildだけを行った。

## 既知の差と修正

| 観点 | 提供C++ | v081 | v086 |
|---|---|---|---|
| rough方策時計 | 静的成分・rank・compact spec生成後に開始 | main開始時から前計算を算入 | 前計算後に開始し、main時計は1.88/1.90の安全停止に限定 |
| `same_region` | 永続stamp配列 | 呼出しごとに2500要素を初期化 | 永続stamp配列 |
| growth heap | 容量16384の固定heap | 呼出しごとにheapを確保 | 容量16384を一度確保して再利用、満杯時の扱いも固定heap相当 |
| blocker候補 | 上位16件用固定配列 | 全候補Vecを生成してsort | 上位16件用固定配列 |
| pair接触表 | 永続buffer | 呼出しごとに二次元Vecを確保 | 永続buffer |
| FreeState BFS | `cellPool`をqueue兼用 | 別の2500要素queueを作成 | `cell_pool`をqueue兼用 |
| box/hit同点比較 | comparator記載keyだけ | tuple全fieldを比較 | comparator記載keyだけ。ただし同値要素の扱いは後述の通り未再現 |
| moved-first順序 | 2要素固定配列、同人数は先頭維持 | Vec生成と不安定sort | 2要素固定配列、同人数は先頭維持 |
| smooth盤面解除・shape走査 | 既存vectorを参照 | cell列・shape列を複製 | 既存sliceを参照 |

受否式、候補のscore式、quota値、route cutoff、乱数生成は変更していない。一方、C++ `std::sort` / `push_heap` / `pop_heap` とRust `sort_unstable_*` / `BinaryHeap` は、comparatorが同じfieldだけを比較していても、同値要素の順序や有界heapに残る要素集合を再現しない。これは同じ乱数列上でも起き、乱数許容範囲の差とは扱えない。

## 実行前検査

- 1行目のbin名、`#![allow(non_snake_case)]`、新しいtrace keyを照合した。
- hot pathに旧`same_region`の全配列初期化、pairの二次元Vec、growth heapの都度確保が残っていないことを検索した。
- `cargo check --release` と `cargo build --release` をlocal feature有無の両方で通した。

評価では100ケースを一度だけ実行し、スコアに加えて方策時計offset、stamp/heap/pair bufferの発動、route別探索量、CPUを確認する。

## AtCoder 50ケース追試後の訂正

同一AtCoder 50ケースの絶対スコアは、C++が3,676,629,314、旧Rustが3,601,987,793、v086が3,606,239,399だった。v086はC++比-70,389,915（-1.9145%）で、旧Rustから回復したのは+4,251,606（+0.1180%）、元の差の5.696%に過ぎない。したがって、上記監査で確認できたのは選定した既知差の修正であり、end-to-endの挙動同等性ではない。

静的に再照合すると、smoothの時間上限はC++の1.88秒に対して提出Rustが1.90秒であり、これはRust側に約1.06%有利な設定である。従って得点低下の原因ではない。一方、固定smooth比較で観測済みのRust CPU +14.4%はv086のlocal評価でも縮んでおらず、1.90秒の優位を上回るスループット不足が残る。壁時計駆動探索では、この差が同じ数式でも探索回数と行動列を変える。

turn単位の行動、盤面hash、受否閾値、候補列、乱数状態、探索仕事量を比較していないため、最初の意味的分岐はまだ実行軌跡上で特定できていない。v086は「C++忠実移植」ではなく「一部の実行構造をC++へ近づけた版」と訂正する。次の切り分けは、まず時間分岐を固定仕事量へ置き換えて意味的分岐を特定し、次にC++ 1.88秒・Rust 1.90秒の各提出条件で実行できた仕事量を比較する。

## 2026-08-09 再監査: 得点に直結する意味差

### 1. roughの「部分順序 + 打ち切り」

roughには、comparatorが全要素の順序を決めないまま、直後に候補数を制限する箇所が複数ある。

- 空き成分を`compSize`だけでsortし、先頭`compLimit`個だけ探索する。同サイズ成分の順序が異なると、探索する場所が変わる。
- 候補を`L`だけでsortし、先頭`polishIters*4`個だけ形状改良する。同周長候補のどれを高価なpolishへ回すかが変わる。
- compact hit、normal box、peel boxの有界heapは、comparatorが見ないfieldを持つ。`std::push_heap/pop_heap`と`BinaryHeap`の同値要素処理が異なるため、並び順だけでなくheapに残る上位候補集合が変わり得る。
- moved-firstは`(same*10000+overlap, L)`だけでsortし、最初の2候補しか再帰探索しない。

これらは「同得点候補の表示順」ではない。探索する成分、polishする形、保持する箱、再帰に入る配置が変わる。さらにmode 2では選ばれた成分ごとに乱数を1回消費するため、一度の同値順序差が以後全turnの乱数状態と盤面を変える。現時点で最も強い、具体的な意味差候補である。

### 2. 壁時計で変わる探索仕事量

C++とRustが同じ数式を使っても、このsolverは経過時間で`fast_mode`、roughの`adaptive_mode`、relocation開始、polish/peel/moveの中断を切り替える。Rustの1.90秒はC++の1.88秒より有利だが、観測済みのCPU差はそれより大きい。従って比較対象は時間定数ではなく、各切替時点までに完了した候補評価数、region search数、polish数、rollout数である。

静的に見えるRust側の追加コスト候補は、トークンごとの`String`所有化、rough出力の汎用formatting、`polish_region`内の反復`Vec`確保、C++が`Group`に保持する`denom/rawQ`の`powf`再計算、smooth hot pathのclone/境界検査、`statrs::erfc`の多項式実装である。ただし、どれがCPU差の支配項かは静的監査だけでは確定できない。

### 3. 浮動小数点の境界分岐

C++のlibmとRustの`statrs::erfc`、`pow`と`powf/powi`、コンパイラのFMA選択はbitwise一致しない。またC++ roughは入力時に`denom`/`rawQ`を1回計算して保持するが、Rustは複数箇所で再計算する。代数式は同じでも、閾値直上の`>=`、候補scoreのround、順位比較で分岐し得る。これは実在する差だが、現時点では部分順序と仕事量差より優先度を下げる。

## 忠実移植の実施順序

1. 壁時計を一時的に固定仕事量へ置き換え、C++をoracleとする。turn入力、route、受否閾値のbit列、候補生成順、有界heapに残った集合、polish対象、RNG状態、選択行動、盤面hashをturnごとに照合する。
2. roughのcritical pathに限り、AtCoderのC++ toolchainが使う`std::sort` introsortと`push_heap/pop_heap`の要素交換順をRustで再現する。別案の「両言語でtotal keyを追加する」は再現性は高いが、元C++の候補選択を変えるため、先にC++側で得点を保つと確認できなければ忠実移植には使えない。
3. `Group`へC++同様の`denom`/`rawQ`を戻し、浮動小数点関数の計算回数と式順を合わせる。必要な関数だけC++ libmとULP比較し、最初の分岐に関与するものを優先する。
4. 意味的一致後にだけ高速化する。fixed arrayとscratch bufferの再利用、数値scanner、整数output、不要clone削減を行い、候補生成順を変えないことを差分ハーネスで連続確認する。
5. 最後に壁時計を戻し、C++ 1.88秒とRust 1.90秒で、各phase到達回数と完了仕事量を比較する。スコア比較はその後に行う。

この順序なら、「数式は同じだが遅い」と「時間無限でも別の候補を選ぶ」を混同しない。現時点の結論は、v086には両方が残っており、先にroughの部分順序を正すのが最も重要、次に意味を保ったスループット同等化が重要である。
