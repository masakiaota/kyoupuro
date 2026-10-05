# GCC最適化指定とAHC優勝者の公開コード

2026-09-26に、AtCoderの現行コンパイル設定、GCC 15.2の仕様、過去の優勝者の公開C++コードを調査した。solverの実行による速度比較は行っていない。

## 結論

時間いっぱい探索するsolverでは、`O3`やループ展開によって候補評価の回数が増える可能性がある。標準の`O2`でコンパイルできることだけでは、速度面の検討は済まない。ソース内のpragmaによる最適化指定は、実際にAHC優勝者も使っている。

ただし、優勝コードで使われている事実は、その指定単独の速度向上を示す測定ではない。採否は対象solverでの候補評価速度とスコアで判断する。

## 現行のAtCoder設定

[AtCoderの言語一覧](https://img.atcoder.jp/file/language-update/2025-10/language-list.html)で確認したC++23（GCC 15.2.0）の関連部分は次のとおりである。

```text
-std=gnu++23 -O2 -march=native -Wall -Wextra
-ftrivial-auto-var-init=zero -pthread -fopenmp
-DATCODER -DONLINE_JUDGE -DNOMINMAX
```

`scripts/build_solver.sh`も、最適化に関係するこの設定に合わせてある。AtCoderにはすでに`-march=native`があるため、明示的な`target`指定が利用可能な命令をさらに増やすとは限らない。

`#pragma GCC optimize`は、後続で定義される関数に最適化オプションを指定する。コンパイルコマンド全体を`-O3`に変更することと完全に同義ではない。ヘッダー内の関数定義にも適用する書き方では、`#include`より前に置く。

## 優勝者の公開コードで確認した指定

| 大会・作者 | 確認した指定 | 公開コードの位置づけ |
|---|---|---|
| AHC017・bowwowforeach | `O3,omit-frame-pointer,inline`、`unroll-loops`、`target("avx2")` | 最終提出をリファクタリングし、コメントを加えた解説用コード |
| AHC018・Psyho | `Ofast,omit-frame-pointer,inline,unroll-all-loops` | 作者が最終提出として公開したコード |
| World Tour Finals 2025 Heuristic・Psyho | `Ofast,omit-frame-pointer,inline,unroll-all-loops` | 作者が最終提出からデッドコードを除いて公開したコード |

出典は以下である。

- AHC017：[優勝を明記した作者の参加記](https://bowwowforeach.hatenablog.com/entry/2023/02/09/210819)、[解説用コード](https://atcoder.jp/contests/ahc017/submissions/38692385)。
- AHC018：[作者の解法と最終提出へのリンク](https://github.com/FakePsyho/cpcontests/blob/master/atcoder/ahc018/approach.md)、[コード](https://github.com/FakePsyho/cpcontests/blob/master/atcoder/ahc018/main.cpp)。順位は[大会まとめ](https://jetbead.github.io/AtCoderHeuristicContestMemo/ContestMemo/ahc018.html)でも確認した。
- World Tour Finals 2025：[作者の大会回顧](https://github.com/FakePsyho/cpcontests/blob/master/atcoder/awtf2025/humansvsai.md)、[解法](https://github.com/FakePsyho/cpcontests/blob/master/atcoder/awtf2025/approach.md)、[コード](https://github.com/FakePsyho/cpcontests/blob/master/atcoder/awtf2025/main.cpp)。

## 提示されたpragmaの読み方

```cpp
#pragma GCC optimize("O3,omit-frame-pointer,inline,unroll-loops,fast-math")
#pragma GCC target("avx2,bmi,bmi2,lzcnt,popcnt")
```

| 指定 | 役割と評価時に見る点 |
|---|---|
| `O3` | `O2`に追加のループ変換などを加え、ベクトル化の費用判断も変える。速度改善の有力候補だが、すべての処理で速くなる保証はない。 |
| `omit-frame-pointer` | フレームポインターを可能な関数で省略する。GCCでは`O1`以上で既定有効なので、通常は追加の改善ではない。 |
| `inline` | インライン展開を指定する。GCC 15の`O2`にも主要なインライン展開最適化が含まれ、全関数への強制展開を意味しない。 |
| `unroll-loops` | ループを展開する。繰り返しの制御や命令依存を減らせる反面、コード量の増大によって遅くなる場合もある。 |
| `fast-math` | 浮動小数点演算の厳密な意味を一部緩める。整数計算そのものを高速化する指定ではない。 |
| `avx2` | SIMD命令を利用可能にする。指定だけで任意のループがベクトル化されるわけではない。 |
| `bmi,bmi2,lzcnt,popcnt` | ビット操作用の命令を利用可能にする。`-march=native`で有効になる機能との重複を確認する。 |

`Ofast`には`O3`と`fast-math`に加えて別の設定も含まれるため、`O3,fast-math`と完全に同じではない。

仕様の出典：[GCC 15.2 Optimize Options](https://gcc.gnu.org/onlinedocs/gcc-15.2.0/gcc/Optimize-Options.html)、[Function-Specific Option Pragmas](https://gcc.gnu.org/onlinedocs/gcc-15.2.0/gcc/Function-Specific-Option-Pragmas.html)。

## 提示コードの実装から学ぶ点

ユーザーが提示したレンガ配置のコードは、出典や順位が与えられていないため、上記の優勝者のコードとは別の例として扱う。

- **状態の圧縮**：配置を2本の`uint64_t`に符号化し、探索状態を`static_assert(sizeof(State)==32)`で32バイトに収めている。状態のコピー量とメモリ帯域の負担を抑える。
- **ビット集合の一括処理**：必要なマスを64ビットにまとめ、集合演算、`popcount`、`ctz`で処理している。命令セット指定と実際のデータ表現が対応している。
- **初期化の削減**：重複排除のハッシュ表を世代番号で管理し、毎回の全要素クリアを避けている。
- **必要な順位だけ選ぶ**：枝刈りに`nth_element`を使い、全候補の整列を省く。全ソートの`O(n log n)`に対して、選択処理は平均線形時間になる。
- **確定した処理の前計算**：帯DPの状態遷移を構築時に列挙し、DPの各位置では前計算済みの辺を走査する。
- **作業領域の再利用**：DP配列や候補バッファを使い回し、履歴は親IDで保持する。状態ごとの履歴全コピーを避ける。

この例の重い処理は主として整数演算である。`fast-math`が速度の主因だとは、ソースの存在だけから判断できない。pragmaとともに、何をコピーし、初期化し、走査しているかを見る必要がある。

## v006への適用判断

調査開始時の`src/bin/v006_branch_reconnect.cpp`にはpragmaがなかった。最終確認時には、上記と同じ2行がファイル先頭のコメント直後に追加されていた。この調査でsolverは編集していない。

v006の操作数評価と合法性判定は整数が中心である。浮動小数点は焼きなましの温度、`pow`、受理確率の`exp`、時刻の計算などに使われる。`fast-math`は検討に値するが、受理判定が変化する可能性もあるため、従来の評価結果から追加後のスコアや速度を推定しない。

このPCは`uname -m`で`arm64`と確認した。x86専用の`target`指定を含むソースをローカルGCCでもビルドするなら、命令セット指定をアーキテクチャで分岐させる。

```cpp
// ファイル名コメントの次、includeの前に置く。
#pragma GCC optimize("O3,omit-frame-pointer,inline,unroll-loops,fast-math")
#if defined(__x86_64__)
#pragma GCC target("avx2,bmi,bmi2,lzcnt,popcnt")
#endif
#include <bits/stdc++.h>
```

これはGCCを前提にした書き方である。ARM上での測定は、AtCoderのx86向け命令生成を直接検証するものではない。

次の実験候補は、最適化指定による候補評価速度とスコアの比較である。アルゴリズムや時間配分は固定し、`O3`、ループ展開、`fast-math`のどこから効果が出たかを混同しない構成を事前登録する。O14の多量の反復評価と、O15の走査量だけでは速度を決められなかった観測が根拠となる。backlogのB-11に対応する調査である。
