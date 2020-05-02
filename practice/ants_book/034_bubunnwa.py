# 指数時間でなくても解けそうだけどどうやるんだろうか...
n = 4
a = [1, 2, 4, 7]
k = 13

import sys
sys.setrecursionlimit(1 << 25)


def dfs(i, s):  # 足し算の状態を下に伝播して、作れるかの状態を上に伝播する
    # 終了条件
    if i == n:
        return s == k  # 合計がkになってればok
    # 使わなかった場合の探索
    flg1 = dfs(i + 1, s)
    # 使う場合の探索
    s += a[i]
    flg2 = dfs(i + 1, s)

    return flg1 or flg2  # どっちかがTrueならば良い


print('Yes' if dfs(0, 0) else 'No')
