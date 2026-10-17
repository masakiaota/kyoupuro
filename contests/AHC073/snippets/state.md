# 盤面をクラスで持つ

1. [state.py](state.py) の `class State:` から末尾までをコピーする。
2. 解法の `main.py` にある `read_input()` より前に貼る。
3. 盤面の初期化、評価値、操作を問題に合わせて変える。

例は、整数の盤面を持ち、`(行, 列, 新しい値)` で1マスを書き換える。評価値は盤面の合計とする。

## 作る・読む・変える

```python
state = State([[1, 2], [3, 4]])  # __init__ が呼ばれる
print(state.grid[0][1])          # 2
print(state.value)               # 10

candidate = state.copy()         # 独立した候補を作る
undo_info = candidate.apply((0, 1, 9))
print(candidate.grid[0][1])      # 9
print(candidate.value)           # 17
print(state.grid[0][1])          # 元の状態は 2 のまま

candidate.undo(undo_info)        # 変更前へ戻す
print(candidate.value)           # 10
```

| 書き方 | 意味 |
| --- | --- |
| `State(grid)` | 新しい状態を作る。その際に `__init__` が実行される |
| `self` | そのメソッドを呼んだ状態自身。`state.apply(...)` の中では `self` が `state` を指す |
| `self.grid` | その状態が持つ盤面。クラスの外からは `state.grid` と書く |
| `state.copy()` | 元を残したまま別の候補を作る |
| `state.apply(action)` | その状態を書き換え、取り消し用の情報を返す |
| `state.undo(undo_info)` | 最後の変更を取り消す。複数の変更は逆順に戻す |

`other = state` は同じ状態を指す。独立した候補が必要なら `state.copy()` を使う。

## 探索の骨格につなぐ

以下は `read_input()` が `N, M, grid` を返す場合。`State` は盤面の大きさを `grid` から読む。

```python
def make_initial(data):
    N, M, grid = data
    return State(grid)


def score(data, state):
    return state.value
```

### 焼きなまし：候補をコピーする版

`annealing.py` の `neighbor()` を次の形にする。例ではランダムな1マスを0か1に変える。

```python
def neighbor(data, state, rng):
    candidate = state.copy()
    r = rng.randrange(state.h)
    c = rng.randrange(state.w)
    candidate.apply((r, c, rng.randrange(2)))
    return candidate
```

### 焼きなまし：変更して取り消す版

`annealing_delta.py` では `copy_state()` をそのまま使い、次の2関数を埋める。

```python
def apply_random_change(data, state, rng):
    before = state.value
    r = rng.randrange(state.h)
    c = rng.randrange(state.w)
    undo_info = state.apply((r, c, rng.randrange(2)))
    return state.value - before, undo_info


def undo_change(data, state, undo_info):
    state.undo(undo_info)
```

### 貪欲法・ビームサーチ

`expand()` の中で `child = state.copy()`、`child.apply(action)`、`yield child` と書く。比較には `child.value` を使える。

盤面そのものを出力する形式なら、`write_output()` は次の形になる。

```python
def write_output(answer):
    for row in answer.grid:
        print(*row)
```

## 問題に合わせて変える場所

- **評価値**：`__init__` の初期計算と、`apply` の差分計算を変える。
- **操作**：`apply` で変更した値を取り消し情報に入れ、`undo` で戻す。
- **持たせる値**：位置、訪問済み、操作列など、必要なものを `self.名前` に追加する。`copy` で引き継ぎ、配列なら複製する。`apply` で変える値は `undo` でも戻す。
- **固定の入力**：変更しない重みや隣接情報は `data` から参照し、状態ごとにはコピーしない。
- **出力する操作列**：盤面の配置を改善する際に試した変更と、提出すべき操作列を区別する。状態ごとに履歴を持つなら、その履歴も `copy` と `undo` の対象にする。
