# https://atcoder.jp/contests/arc026/tasks/arc026_2
# 約数高速列挙かな


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
s = sum(make_divisors(N)) - N
if N == s:
    print('Perfect')
elif N > s:
    print('Deficient')
else:
    print('Abundant')
