---
name: make-v000-template
description: C++ を使う AHC プロジェクトで problem_description.txt を読み、v000_template.cpp に入出力・State・操作適用の共通土台をユーザーと段階的に合意して実装する。solver 戦略ではなく、問題ルールを安全かつ高速に扱う基盤設計を支援する。
---

# make-v000-template

AHC の `v000_template.cpp` に、問題固有だが solver 戦略に依存しない共通土台を作る。

## 記号と命名

- `v000_template.cpp` の 1 行目に `// v000_template.cpp` を置く。
- 入出力、`Input` のメンバー、局所変数を設計するときは、`notes/notations.md` で定めた公式記号を同じ綴りで使う。対応づけだけを目的に `N` を `n`、`M` を `m` へ変換しない。
- 問題文にない実装用の状態量、関数、局所変数は通常どおり C++ の `snake_case` にする。
- `v000_template.cpp` を複製して作る solver も、単独で AtCoder へ提出できる一ファイル構成にする。

## 進め方

1. `problem_description.txt`、`notes/notations.md`、既存の `src/bin/v000_template.cpp`、`scripts/build_solver.sh` を読む。

2. 実装せず、ユーザーへ入出力の `struct` と関数の簡潔な設計案を示す。

3. ユーザーが許可した段階だけ `v000_template.cpp` に反映する。

4. 実装せず、ユーザーへ状態 `State` の簡潔な設計案を示す。
   - 現在位置、盤面、手持ち、残量、スコアに必要な量を洗い出す。
   - 状態更新で高速に参照したいものと、探索戦略に依存するものを分ける。
   - `vector`、`array`、bitset、隣接表などの候補は制約と hot loop を踏まえて比較する。
   - hash、undo、探索用 metadata は必要性が明確になるまで入れない。

5. ユーザーが許可した段階だけ `v000_template.cpp` に反映する。

6. 実装せず、ユーザーへ操作 `State::apply` または `apply()` の簡潔な設計案を示す。
   - 操作の合法性、状態変化、スコア関連量の更新を明確にする。
   - 状態更新の入口は増やしすぎず、メンバー関数にするか関数にするかを問題ごとに判断して提案する。
   - hot loop になる前提なら、移動先表・近傍表・差分更新・高速版の必要性を検討する。
   - 不合法操作は基本的に生成しない前提で、検証用に検出するか、高速適用に寄せるかを用途に合わせて決める。

7. ユーザーが許可した段階だけ `v000_template.cpp` に反映する。

8. 各反映後に `./scripts/build_solver.sh v000_template` を実行する。

## しないこと

- solver 候補や探索戦略を勝手に作らない。
- ユーザーが許可していない段階まで `v000_template.cpp` を先回りして実装しない。
- `notes/notations.md` や `notes/important_properties.md` を、明示指示なしに更新しない。
