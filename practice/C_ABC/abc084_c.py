# https://atcoder.jp/contests/abc084/tasks/abc084_c
# 制約によりたかだか500行 O(N*N)でも間に合うのでは？

# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_a_int():
    return int(read())


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


N = read_a_int()
C, S, F = read_col(N - 1, 3)


def f(i):
    # 駅iからN-1までに向かう最短時間
    now = S[i]  # はじめは運転開始時刻
    for j in range(i, N - 2):  # 駅N-1の手前まで
        now += C[j]  # 移動時間
        # 駅の待ち時刻まで待機
        f, s = F[j + 1], S[j + 1]
        # だけど割り切れるときは余分に＋1されてしまうので-0.5してつじつま合わせ
        now_tmp = (int((now - 0.5) // f) + 1) * f
        now = max(now_tmp, s)
    now += C[N - 2]
    return now


for i in range(N - 1):
    print(f(i))
print(0)
