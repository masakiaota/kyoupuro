# https://atcoder.jp/contests/abc085/tasks/abc085_d
# Hダメージ以上与えたい
# ダメージが大きい方から与えたいが、武器が消えると困るときもある

# 投げつける選択は最後でいい。(ダメージも一番大きい)
# 通常攻撃で一番ダメージが大きいのを連発したいし、倒せるギリギリになったら
# 通常攻撃よりもダメージが大きい投げつけ攻撃を行いたい
# 入力が10**5とかになったときに100ms程度早い
import sys
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


def bisect_left_reverse(a, x):
    '''
    reverseにソートされたlist aに対してxを挿入できるidxを返す。
    xが存在する場合には一番左側のidxとなる。
    '''
    if a[0] <= x:
        return 0
    if x < a[-1]:
        return len(a)
    # 二分探索
    ok = len(a) - 1
    ng = 0
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if a[mid] <= x:
            ok = mid
        else:
            ng = mid
    return ok


N, H = read_ints()
A, B = read_col(N, 2)
a_ma = max(A)
B.sort(reverse=True)
B_use = B[:bisect_left_reverse(B, a_ma)]

# 投げつけ攻撃だけで倒せる場合
cnt = 0
for b in B_use:
    H -= b
    cnt += 1
    if H <= 0:  # 投げつけ攻撃だけで倒せる場合
        print(cnt)
        exit()

# 投げつけ攻撃だけで倒せない場合
ans = len(B_use)
ans += int((H - 0.5) // a_ma) + 1

print(ans)
