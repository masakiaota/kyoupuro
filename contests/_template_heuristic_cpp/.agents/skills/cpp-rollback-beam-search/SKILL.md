---
name: cpp-rollback-beam-search
description: C++ の AHC solver に差分更新型ビームサーチを導入する。Euler Tour 辺走査、二重連鎖木走査、可変ターン rollback の3種類の単一ファイルテンプレートから問題に合うものを選び、Action・State・評価・重複排除を実装するときに使う。
---

# cpp-rollback-beam-search

候補ごとに `State` をコピーせず、1つの `State` を `move_forward(action)` と `move_backward(action)` で進退させるビームサーチを実装する。

## テンプレート選択

- 固定ターンでは `assets/cpp/euler_tour_edges_rollback_beam.cpp` を使う。
- 木の `first_child` / `next_sibling` を直接たどる構造が必要なら `assets/cpp/linked_tree_rollback_beam.cpp` を使う。
- 1つの `Action` が複数ターン進むなら `assets/cpp/variable_turn_rollback_beam.cpp` を使う。

迷ったら Euler Tour 辺列版から始める。

## 導入手順

solver にテンプレートをコピーし、1行目のファイル名コメントを合わせる。

```sh
SKILL_DIR=".agents/skills/cpp-rollback-beam-search"
solver_name="v001_rollback_beam"
cp "$SKILL_DIR/assets/cpp/euler_tour_edges_rollback_beam.cpp" "src/bin/${solver_name}.cpp"
```

既存 solver に組み込む場合は、必要な型と関数だけを貼り込む。

## 問題固有に置き換える箇所

`TODO(problem)` を検索し、最低限次を実装する。

- `Action`: 1手と、逆操作に必要な情報を小さく保持する。
- `State::enumerate_actions`: 現在状態から有望な合法手を列挙する。
- `State::move_forward`: Action を適用して差分管理値を更新する。
- `State::move_backward`: 同じ Action を使って状態を完全に戻す。
- `State::evaluate`: 小さいほど良い `Evaluator` を返す。
- `State::hash_key`: 重複排除したい粒度の key を返す。
- 可変ターン版の `Action::step`: 必ず1以上を返す。

## 原則

- `State` は候補ごとにコピーしない。
- 戻すための情報は原則として `Action` に含める。
- `Evaluator` は全順序を持たせ、小さいほど良い評価に統一する。
- 最大化問題では `score_key = -score` のように符号反転する。
- `hash_key` と `evaluate` は `move_forward` 後の状態から求める。
- beam 幅を広げる前に、全候補で forward / backward が完全な逆操作になることを確認する。

設計の背景は [design-notes.md](references/design-notes.md)、置き換え例は [examples.md](references/examples.md) を読む。
