# AWC0001Beta 作業手順 (uv + oj)

## 制約
- AtCoderに取り組む。これはAI利用が許された試験的なコンテストである。
- 提出言語は Python (CPython 3.13.7) である。
- 解答コードは標準ライブラリのみを使う。ただし `numpy==2.2.6`, `networkx==3.5`, `numba==0.61.2` は使用可能である。
- ローカル実行は常に `uv run ...` を使う (システムの `python` を直接使わない)。

## 事前準備 (初回のみ) (済)
```bash
cd /Users/masaki/Documents/kyoupuro/contests/AWC0001Beta
uv sync
uv run oj --version
```

## AtCoderログイン (必要なとき) (済)
ログイン確認:
```bash
uv run oj login --check https://atcoder.jp/
```

未ログインなら `REVEL_SESSION` を cookie.jar に入れる。
```bash
cd /Users/masaki/Documents/kyoupuro/contests/AWC0001Beta
uv run --with aclogin aclogin --tools oj
uv run oj login --check https://atcoder.jp/
```

`REVEL_SESSION` の取り方:
- ブラウザで AtCoder にログインする
- DevTools の Cookies から `https://atcoder.jp` の `REVEL_SESSION` の値をコピーする
- 上の `aclogin` に貼り付ける

## URLが与えられたときのフロー
目標:
1. `oj` によるサンプル取得
2. 問題文取得 (`statement.txt`)
3. Python実装 (`main.py`)
4. `oj` によるテスト検証 (失敗したら 3 に戻る)
5. 提出 (手動)

### 1-2-3: セットアップ (推奨: スクリプト)
```bash
cd /Users/masaki/Documents/kyoupuro/contests/AWC0001Beta
URL='https://atcoder.jp/contests/.../tasks/...'
DIR='a'  # 例: a, b, practice_a, task_id など
uv run python tools/prepare_problem.py "$URL" --dir "$DIR"
```

このスクリプトが行うこと:
- `DIR/test/sample-*.in/out` を `oj d URL` で生成する (既存の `sample-*` は再取得のため削除してから実行する)
- 問題文を `DIR/statement.txt` に保存する
- `DIR/main.py` が無ければテンプレを生成する

### 4: テスト
```bash
cd /Users/masaki/Documents/kyoupuro/contests/AWC0001Beta/$DIR
uv run oj t -c "python main.py"
```

`WA/RE/TLE` なら `main.py` を修正して `oj t` を再実行する。
追加の自作テストは `test/custom-1.in` と `test/custom-1.out` のように置く。

### 5: 提出 (手動)
AtCoder の提出フォームに Cloudflare Turnstile が入っており、`oj submit` / `oj-api submit-code` の自動提出は失敗することがある。
したがって提出はブラウザ手動で行う。

- 提出ページ: `https://atcoder.jp/contests/<contest>/submit?taskScreenName=<task>`
- 言語: `Python (CPython 3.13.7)` を選ぶ
- `main.py` の内容を貼り付けて Submit
