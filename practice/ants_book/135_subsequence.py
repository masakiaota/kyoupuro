# この問題を見たらしゃくとり法よりも累積和を使いたくなるな


def two_pointers(ls: list, S):
    n_ls = len(ls)
    ret = []

    r = 0
    s = 0
    for l in range(n_ls):
        while r < n_ls and s + ls[r] < S:  # 初めて条件を満たす一歩手前をr)にする。
            s += ls[r]
            r += 1
        ret.append((l, r))
        if r == n_ls:
            break
        # 抜けるときの更新
        s -= ls[l]
    return ret


def solve(n, S, A):
    idxs = two_pointers(A, S)
    print(idxs)
    if len(idxs) == 1:
        print(0)  # S以上にすることはできない
    else:
        print(min([r - l + 1 for l, r in idxs]))


# 入力例1
n = 10
S = 15
A = [5, 1, 3, 5, 10, 7, 4, 9, 2, 8]
solve(n, S, A)

# 入力例2
n = 5
S = 11
A = [1, 2, 3, 4, 5]
solve(n, S, A)

# 入力例 オリジナル (すべてがS以上)
n = 5
S = 1
A = [3, 2, 3, 4, 5]
solve(n, S, A)

# 入力例 オリジナル (S以上になれない)
n = 5
S = 100
A = [1, 2, 3, 4, 5]
solve(n, S, A)

# 入力例 オリジナル (ちょうどS)
n = 5
A = [1, 2, 3, 4, 5]
S = sum(A)
solve(n, S, A)
