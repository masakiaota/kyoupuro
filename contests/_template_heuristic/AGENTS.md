# Heuristic Contest Agent Notes

## 前提
- このディレクトリがプロジェクトルートである。親ディレクトリや兄弟ディレクトリには依存しない。
- 言語は Rust である。
- 現在の依存環境以外は用いない。(AtCoderのジャッジ環境制約)
- 提出するのは `src/bin/*.rs` 側のファイルを想定する。コピペで提出できるように 1 ファイルで完結させる。
- 提出候補は `src/bin/*.rs` に複数保持してよい。提出時は対象ファイルをユーザーが直接コピーして使う。
- 問題文や要点は `problem_description.txt` に記録する。
- 公式配布物は `tools/` に配置する。
- visualizer 実装は `.agents/skills/make-visualizer/SKILL.md` に従う。

## ディレクトリの役割
- `src/bin/*.rs`
  - 実験用コード兼、提出候補を置く。各ファイルは単体で完結させる。
- `results/scores.csv`
  - 実行ログを蓄積する。
- `notes/`
  - アイデアや性質のメモを残す。
- `tools/`
  - 公式 tester や generator を展開する。
- `wasm/src/impl_vis.rs`
  - 問題固有の visualizer ロジックを実装する。
- `public/wasm/`
  - `wasm-pack` の生成物を置く。

## 注意
- 公式 tools の構成は contest ごとに異なる。共通 wrapper script はあるが、引数や bin 名は必要に応じて調整してよい。
- `public/wasm/` は `wasm-pack` の生成先である。`wasm` 側を変更したら `./scripts/build_wasm.sh` を実行する。
