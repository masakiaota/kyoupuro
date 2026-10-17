# 以下のテンプレートを使用
# https://github.com/masakiaota/kyoupuro/tree/9afb105b7761edfd67f158672e5c47caa90510f8/contests/AHC073
"""整数盤面の状態。class State 全体を main.py の read_input より前に貼る。

例は「1マスの値を書き換え、盤面の合計を評価値にする」操作。
問題に合わせて __init__、apply、undo の中身を変える。使い方は state.md。
"""


class State:
    # grid は整数の2次元リスト。value は copy() が引き継ぐ評価値。
    def __init__(self, grid, value=None):
        self.h = len(grid)
        self.w = len(grid[0])
        self.grid = [row[:] for row in grid]
        self.value = sum(map(sum, grid)) if value is None else value

    # 行もコピーするので、元の状態と盤面を共有しない。
    def copy(self):
        return State(self.grid, self.value)

    # action = (行, 列, 新しい値)。書き換え前の情報を返す。
    def apply(self, action):
        r, c, new_value = action
        old_value = self.grid[r][c]
        undo_info = (r, c, old_value, self.value)
        self.grid[r][c] = new_value
        self.value += new_value - old_value
        return undo_info

    # 最後に apply した変更から、逆順に戻す。
    def undo(self, undo_info):
        r, c, old_value, old_score = undo_info
        self.grid[r][c] = old_value
        self.value = old_score
