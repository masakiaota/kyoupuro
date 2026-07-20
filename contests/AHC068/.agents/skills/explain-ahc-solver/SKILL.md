---
name: explain-ahc-solver
description: AHC形式のRust solverから、日本語の再現可能な解説を作る。入力としてチャンピオン実装や提出候補コードが与えられたときに、Phase 1で削除や簡素化により本質を抽出し、Phase 2でjapanese-tech-writingに従う解説を書き、subagentに解説だけから再実装と評価をさせて品質を検証する。
---

# explain-ahc-solver

AHC solver の実装から、読者が方針を再現できる日本語解説を作る。

この skill は、実装をそのまま説明するためのものではない。
まず得点と時間設計を保った簡素化 solver を作り、そこから解説すべき本質を確定する。
その後、その本質を文章に落とし込み、事情を知らない subagent が解説だけから十分な solver を再実装できるかを検証する。

Phase 1 の完成 solver は、Phase 2 を通じて固定された正本かつベンチマークである。
Phase 2 の subagent solver は、解説の品質を測るための使い捨てプローブである。
subagent solver を新しいベースライン、改善対象、チューニング対象にしてはならない。

## ワークフロー

この skill は、次の2つの phase を順に実行する。

1. **Phase 1：実装から本質を抽出する**
   対象 solver を削除や簡素化で整理し、得点と時間設計を保つ簡素化 solver を作る。
   手順は [simplification.md](references/simplification.md) に従う。

2. **Phase 2：解説を作り、再現性を検証する**
   Phase 1 の完成 solver を正本として、抽出した本質を日本語解説へ落とし込み、subagent による再実装で解説の品質を検証する。
   手順は [explanation-validation.md](references/explanation-validation.md) に従う。

Phase 1 が完了するまで、Phase 2 に入らない。
ただし、ユーザーがすでに Phase 1 相当の簡素化 solver と本質抽出メモを与えた場合は、それを Phase 1 の成果物として扱ってよい。

## 必ず読むもの

作業の入口では、次を読む。

1. `problem_description.txt`
2. `notes/notations.md`
3. 対象 solver
4. `src/bin/v000_template.rs`
5. 評価に使う `scripts/eval.py`, `scripts/run.sh`, `Cargo.toml`

Phase 2 に入る前に、`japanese-tech-writing` を読む。
解説本文は、その規範に従って一文一行で書く。
Phase 2 では、subagent に渡す前に [writing-checklist.md](references/writing-checklist.md) で検証対象本文を確定する。
subagent 検証を実行するときは、[subagent-validation.md](references/subagent-validation.md) のプロンプトと読み取り制限に従う。

## Phase 1 の完了条件

Phase 1 は、入力 solver を読んで分かった気になるだけでは完了しない。

次の状態になってから Phase 2 に進む。

- 簡素化 solver がある
- 目標スコアを超える評価結果がある
- `max_elapsed` がプロジェクトの時間条件を満たすように設計されている
- 採用した削除と不採用にした削除が記録されている
- 残った構造を解説本文の中心として説明できる
- 削れた処理を解説本文から落としてよい理由が分かっている

詳しい反復手順、停止条件、時間超過への対応は [simplification.md](references/simplification.md) に従う。

## Phase 2 の完了条件

Phase 2 は、解説 Markdown を書いただけでは完了しない。

次の状態になってから成果物とする。

- 解説本文が `japanese-tech-writing` に従っている
- 解説本文が、何をするかだけでなく、なぜ必要かを説明している
- 実装細部を書きすぎず、本質のアイデアを中心にしている
- 新規 subagent が、v000 以外の既存 solver を読まずに再実装している
- subagent の再実装が目標スコアを超えている
- subagent の結果を、固定した Phase 1 の完成 solver と比較している
- subagent の成功が過度な試行錯誤や評価後チューニングに依存していない
- subagent solver を修正、再評価、次の検証のベースとして利用していない
- 最終本文が subagent の読んだ検証対象本文と同一である、または意味を変えない編集だけである

詳しい文章作成、検証、失敗時の扱い、再検証、最終確認は [explanation-validation.md](references/explanation-validation.md) に従う。
subagent に渡す前の本文点検には [writing-checklist.md](references/writing-checklist.md) を使う。
subagent を起動するときは [subagent-validation.md](references/subagent-validation.md) を使う。
検証 agent には、読んでよいファイル、作ってよい新規ファイル、読んではいけない既存 solver を明示する。

## 参照ファイル

- [simplification.md](references/simplification.md)：Phase 1 の手順。
- [explanation-validation.md](references/explanation-validation.md)：Phase 2 の手順。
- [writing-checklist.md](references/writing-checklist.md)：Phase 2 で subagent に渡す前の解説本文を、AHC solver 解説として検証可能な状態にするための点検手順。
- [subagent-validation.md](references/subagent-validation.md)：subagent 起動時に使う検証プロンプト、読み取り制限、checksum、検証条件の汚染防止の手順。

## 最終報告

最後に、次を簡潔に報告する。

- Phase 1 の簡素化 solver、評価コマンド、スコア、時間設計の確認
- Phase 1 で採用した削除と不採用にした削除
- Phase 2 の解説ファイル
- subagent 検証の有無、再実装 solver、評価コマンド、スコア、Phase 1 の完成 solver との比較
- subagent が読んだ解説ファイル、可能なら checksum、最終解説ファイルとの同一性
- 解説に残る曖昧点
- 本文へ反映した改善点
- 反映しなかったパラメータやケース固有情報と、その理由
