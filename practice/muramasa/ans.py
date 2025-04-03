import copy
from pprint import pprint

# 与えられた候補リストと目標合計
candi = [62, 9, 41, 49, 105, 96, 8, 22, 101, 55, 35, 46, 84, 60, 99, 24, 74, 124]
target_sum = 315

# tensor の定義（各レベルは 5×5 のマジックスクエアで、レベル４・５の一部が None）
tensor = [
    [
        [25, 16, 80, 104, 90],
        [115, 98, 4, 1, 97],
        [42, 111, 85, 2, 75],
        [66, 72, 27, 102, 48],
        [67, 18, 119, 106, 5],
    ],
    [
        [91, 77, 71, 6, 70],
        [52, 64, 117, 69, 13],
        [30, 118, 21, 123, 23],
        [26, 39, 92, 44, 114],
        [116, 17, 14, 73, 95],
    ],
    [
        [47, 61, 45, 76, 86],
        [107, 43, 38, 33, 94],
        [89, 68, 63, 58, 37],
        [32, 93, 88, 83, 19],
        [40, 50, 81, 65, 79],
    ],
    [
        [31, 53, 112, 109, 10],
        [12, 82, 34, 87, 100],
        [103, 3, None, None, None],
        [113, 57, None, None, None],
        [56, 120, None, None, None],
    ],
    [
        [121, 108, 7, 20, 59],
        [29, 28, 122, 125, 11],
        [51, 15, None, None, None],
        [78, 54, None, None, None],
        [36, 110, None, None, None],
    ],
]

# 未定義セル（None）の場所をリスト化
# レベル４（tensor[3]）とレベル５（tensor[4]）の各行・各列に存在する None の座標を (level, row, col) の形式で保持
missing_positions = []
for level in [3, 4]:
    for i in range(5):
        for j in range(5):
            if tensor[level][i][j] is None:
                missing_positions.append((level, i, j))


# 与えられた「線」（行・列・対角線）の部分列について、既に入っている値の和と、残りセルの数から
# 残りに global から取れる最小／最大の合計で target_sum に到達可能かどうかをチェックする関数
def check_line(line, available):
    current_sum = sum(x for x in line if x is not None)
    missing_count = line.count(None)
    # 全て埋まっていれば、正確に target_sum であることを確認
    if missing_count == 0:
        return current_sum == target_sum
    # available 内の最小／最大の値を使った場合の合計で、到達可能かどうかを判定
    sorted_avail = sorted(available)
    if len(sorted_avail) < missing_count:
        return False
    min_possible = sum(sorted_avail[:missing_count])
    max_possible = sum(sorted_avail[-missing_count:])
    return (current_sum + min_possible <= target_sum) and (
        current_sum + max_possible >= target_sum
    )


# あるレベルについて、各行・各列・2つの対角線の部分チェックを行う関数
def check_level(level, available):
    grid = tensor[level]
    # 行のチェック
    for i in range(5):
        row = grid[i]
        if not check_line(row, available):
            return False
    # 列のチェック
    for j in range(5):
        col = [grid[i][j] for i in range(5)]
        if not check_line(col, available):
            return False
    # 主対角線のチェック
    diag = [grid[i][i] for i in range(5)]
    if not check_line(diag, available):
        return False
    # 逆対角線のチェック
    anti = [grid[i][4 - i] for i in range(5)]
    if not check_line(anti, available):
        return False
    return True


solutions = []


# 再帰的なバックトラック探索の関数
def backtrack(index, available):
    # すべての未定義セルに値が埋まったら、両レベルの条件を満たしているか最終チェック
    if index == len(missing_positions):
        if check_level(3, available) and check_level(4, available):
            solutions.append(copy.deepcopy(tensor))
            print("Solution found!")
        return

    level, i, j = missing_positions[index]
    # 利用可能な候補から１つずつ割り当て
    for num in available:
        tensor[level][i][j] = num
        # 今割り当てたレベルについて、部分チェック（割り当て後に利用可能な数字は num を除いたもの）
        if check_level(level, [x for x in available if x != num]):
            backtrack(index + 1, [x for x in available if x != num])
        # 戻す
        tensor[level][i][j] = None


# 探索開始
backtrack(0, candi)

print("Total solutions found:", len(solutions))
pprint(*solutions)
