# https://atcoder.jp/contests/jsc2019-qual/tasks/jsc2019_qual_b

# Aに対する転倒数はすぐ求まる
# Aが繰り返されたときに転倒数はどのように増えるのか？
# Aのn_invは少なくとも*Kされる
# 繰り返しの分で増えるものは(K-1)K/2倍される
# 繰り返しの分はどうやって求める？→2倍にしたAに対して n_inv_2A-2n_invすればよい

import sys
read = sys.stdin.readline
ra = range
enu = enumerate
MOD = 10**9 + 7


def read_ints():
    return list(map(int, read().split()))


def ret_n_inv(ls):
    n_inv = 0
    N = len(ls)
    for i in ra(N):
        for j in ra(i + 1, N):
            if ls[i] > ls[j]:
                n_inv += 1
    return n_inv


N, K = read_ints()
A = read_ints()

# 愚直に転倒数を求める
n_A = ret_n_inv(A)
if K == 1:
    print(n_A)
    exit()
n_2A = ret_n_inv(A + A)
mul = (K - 1) * K // 2 % MOD
n_skip = (n_2A - 2 * n_A) * mul % MOD
print((n_A * K + n_skip) % MOD)
