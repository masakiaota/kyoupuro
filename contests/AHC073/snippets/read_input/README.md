# 入力テンプレート

1. 入力の形に合うファイルを下表から開く。
2. `def read_input():` から関数末尾までをコピーし、解法の `read_input()` 全体と置き換える。必要な import も関数内にある。
3. 先頭行の変数と読む行数を問題文に合わせる。戻り値の順番は、各ファイルの先頭に記載している。

| 入力の形 | コピー元 |
| --- | --- |
| 整数の列が1行だけ | [one_line.py](one_line.py) |
| 件数 `N` の後に整数の行が `N` 行 | [records.py](records.py) |
| `N M` の後に整数の盤面が `N` 行 | [int_grid.py](int_grid.py) |
| `N K` の後に空白なしの文字盤面が `N` 行 | [string_grid.py](string_grid.py) |
| 先頭に件数がなく、固定数の行が続く | [fixed_rows.py](fixed_rows.py) |
| 開始位置の後に整数の盤面が2枚 | [two_grids.py](two_grids.py) |
| 座標、辺、座標のように複数のブロックが続く | [blocks.py](blocks.py) |
| 小数を含む先頭行と、行ごとに長さが変わる座標列 | [mixed.py](mixed.py) |
| 通常型で数値を一括読みしたい | [all_tokens.py](all_tokens.py) |
| 対話型で、最初に件数と数値行を受け取る | [interactive.py](interactive.py) |

## 戻り値を使う

たとえば `records.py` は `N, rows` を返す。後続の関数では次のように受け取る。

```python
def greedy(data):
    N, rows = data
    # rows[i] を使って解を作る。
```

先頭行が `N` だけなら `N = int(input())`、`N M` なら `N, M = map(int, input().split())` とする。読む変数を変えたら、戻り値と後続の受け取り方も合わせる。

## 対話型の入力

- `read_input()` は最初に受け取る情報だけを読む。
- 各手番の応答と終了条件は、解法の `read_turn(data)` に書く。整数1個なら `int(input())`、整数2個なら `a, b = map(int, input().split())` で読める。
- 初期入力がない問題では、`read_input()` は `return {}` とする。最初の手番を先に読む場合は、対話型の骨組みを `SEND_FIRST = False` にする。
- `all_tokens.py` の一括読みは、通常型だけに使う。
