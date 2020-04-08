# https://atcoder.jp/contests/diverta2019/tasks/diverta2019_d
# 問題文を数式に起こすと N=am+a (a<m, a \in N)を満たすmの合計を求めろという問題
# N=a(m+1) なのでN%(m+1)=0になるmの中で条件を満たすものを合計すれば良い。つまりNの約数を列挙する。
# 約数は√Nまで調べればいいので、オーダーはO(√N)となる


def make_divisors(n: int, sort=False):
    # 約数列挙
    divisors = []
    for i in range(1, int(n**0.5) + 1):
        if n % i == 0:
            divisors.append(i)
            if i != n // i:
                divisors.append(n // i)
    if sort:
        divisors.sort()
    return divisors


N = int(input())
candi = make_divisors(N)
ans = 0
for mp in candi:
    m = mp - 1
    if m > 0 and (N // m) == (N % m):
        ans += m
print(ans)
