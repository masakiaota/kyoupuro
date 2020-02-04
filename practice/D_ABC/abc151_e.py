# https://atcoder.jp/contests/abc151/tasks/abc151_e
# NCK種のすべてのSについてf(S)の合計をもとめる.
# S1,S2...とすると
# sum_i f(Si) = max(S1) + max(S2) ... max(S NCK) - (min(S1) + min(S2) ... min(S NCK))
# となる。
# sum_i max(Si)について考えると、ある要素を固定したときにそれが最大となるような集合を全パターン持ってくれば簡単に計算できる
# 例えば 1 1 3 4 (K=2)で 4が最大となるとり方は3C1(K-1)
# 3が最大となるとり方は(2C1)
# 1が最大となるとり方は(1C1)
# 1が最大となるとり方は(0C1=0)
# 以上によりsum_i max(Si) = 4 * 3C1 + 2*2C1 + 1*1C1 + 1*0C1となる
# 注意としてiC(K-1)をiについてリストにするときいちいちcombination_modを呼び出すとTLEになるので、一気にここを用意する必要がある。
# 具体的には(K-1)!の逆元を計算を計算しておいて
# 分子についてはかけて割って調節すれば良い？


MOD = 10**9 + 7


def combination_mod(n, r, mod):
    if r > n or n == 0:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N, K = read_ints()
A = read_ints()
A.sort()


# 通りの数を先に計算しておく
k = K - 1
k_fact = 1
for i in range(k):
    k_fact = k_fact * (i + 1) % MOD
k_fact_inv = pow(k_fact, MOD - 2, MOD)
conbi = [0]
correct = []
tmp = 1
for i in range(1, N):
    if i < k:
        conbi.append(0)
        tmp *= i
        if tmp >= MOD:
            tmp %= MOD
    else:
        tmp *= i
        tmp %= MOD
        conbi.append((tmp % MOD) * k_fact_inv)
        tmp *= pow(i - k + 1, MOD - 2, MOD)  # 逆元をかけて割ったことにする。
        # tmp //= (i - k + 1)
    # correct.append(combination_mod(i, k, MOD)) #確認用
# conbi[i]はA_reverse[i]にかけることに対応

# print(conbi)
# print(correct)

ans = 0
# maxを足す
for a, c in zip(A, conbi):
    ans += a * c
    if ans >= MOD:
        ans %= MOD

        # minを引く
for a, c in zip(A[::-1], conbi):
    ans -= a * c
    if ans >= MOD:
        ans %= MOD


print(ans)
