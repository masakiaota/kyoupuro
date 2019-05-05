# https://qiita.com/drken/items/a5e6fe22863b7992efdb#問題-8最長共通部分列-lcs-問題

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret

# dp[i+1][j+1]を文字列S,Tのそれぞれi,j文字まで考慮したときのLCS(最長部分文字列)の長さと定義する。


S = read()[:-1]
T = read()[:-1]
n_S, n_T = len(S), len(T)

dp = [[0 for _ in range(n_T + 1)] for _ in range(n_S + 1)]
# dp[0][0]なら、空の文字列同士を比較している。

# 初期化
# 今回はいらない空の文字列と他の文字列の共通最長は存在しないため。

# dptableの更新
# おなじ文字を付け足す→最長共通文字列の長さが一つ増える
# その他の処理→特に考えなくていい(ただし、dpなので最長のものをとっていく必要がある。)
from itertools import product
for i, j in product(range(n_S), range(n_T)):
    if S[i] == T[j]:
        # 文字が同じだったらLCS(最長部分文字列)が長くなる
        prelcs = dp[i][j] + 1
    else:
        prelcs = dp[i][j]

    dp[i + 1][j + 1] = max(dp[i][j + 1], dp[i + 1][j], prelcs)

print(dp[-1][-1])
# print(*dp, sep='\n')

# 疑問
# 例えば、abcとacに必要な3パターンを考える。
# ab,aでlcsを一つ増やすとabc,acになるのはわかるが
# abc,aで後者にcを付け足すとlcsも一つ増えるはず、なぜ、増えない扱いになっているのかわからない
# 最後の文字だけを見て判断しちゃうと、abbbbbbb,dfbとかでabbb...の部分で何回も+1されてしまうっていうのはわからんでもないけど
# まあS[i] == T[j]の場合の+1が他のパターンよりも大きい値になるので結局大丈夫っていうことなんだろうけど、いまいち納得していない。

# 気持ち的には、同じ文字を付け足すときだけ知りたいよねっていう解釈のほうがスッキリする。
