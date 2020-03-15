# https://atcoder.jp/contests/abc032/tasks/abc032_c
# 累積積作って(作らなくてもいいけど作ると実装が楽)尺取っすかね


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


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


N, K = read_ints()
S = read_col(N, 1)[0]
if 0 in S:
    print(N)
    exit()

# 尺取法で連続する部分列のうち一番長いものを取得する。
r = -1
ans = 0
for l in range(N):
    # r = max(l, r)
    if r <= l:  # explicitに初期cumを決める
        r = l
        cum = S[r]
    while r < N and cum <= K:  # 初めて条件を満たさなくなるところ、というのが半開区間を使う理由
        r += 1
        if r == N:
            break
        cum *= S[r]
    # print(cum, l, r)
    ans = max(ans, r - l)
    cum //= S[l]
print(ans)
