# 問題 https://atcoder.jp/contests/abc103/tasks/abc103_d

# 入力が10**5とかになったときに100ms程度早い
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


N, M = read_ints()

AB = [tuple(map(int, read().split())) for _ in range(M)]

from operator import itemgetter

AB.sort(key=itemgetter(1))  # リスト1は2次元以の配列

# . - . - . - .

i = -1  # いま考慮している島の位置
ans = 0
for a, b in AB:
    # zero-based index
    a -= 1
    b -= 1
    if a < i:
        # 考えなくてもいい(もう満たしている)要望は飛ばす
        # 同じ島からスタートする橋は許容できるので<=ではなく<
        continue
    ans += 1
    i = b

print(ans)
