# https://atcoder.jp/contests/sumitrust2019/tasks/sumitb2019_e

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate
MOD = 10**9 + 7


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
# 重要な考察
# 1, i番目の人がかぶれる帽子は[0,i)における帽子の内訳の中で,A_iに数が等しいものである
# 2, i番目における各帽子の個数は(色はわからないけど)確定できる. ∵A_iに等しい個数の帽子がかぶれるということとは、次はその個数+1個の帽子があるはず

# よってAi、通りの数を取得ansに掛け算。さらに各種帽子の個数を更新してA{i+1}の処理に映れば良い

ans = 1
c = [0, 0, 0]
for a in A:
    ans *= c.count(a)  # 通りの数に加算
    for i in range(3):
        if a == c[i]:
            c[i] += 1
            break
    if ans > MOD:
        ans %= MOD
print(ans)
