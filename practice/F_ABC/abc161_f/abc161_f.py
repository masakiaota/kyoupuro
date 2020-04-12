# https://atcoder.jp/contests/abc161/tasks/abc161_f
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

# 重要な考察
# 1. N%K!=0のものは、ずっと引き続けられる。その結果1になりたいんだからN%K==1の条件を満たすKは答えの候補である(しかも必ず条件を満たす)。
# 2. N%K==0のものも操作ができるので答えの候補である。(ただし条件を満たさない可能性はある)
# 1の候補は(N-1)の約数、2の候補はNの約数なので、O(√N)で探索が可能

# 具体的にはすべての候補のkに対して、割り切れなくなるまでN//=k。N%k==1ならば、答えに加算する。


# 確実にカウントできる1から
ans = len(make_divisors(N - 1)) - 1  # 1は答えに入らない

# 2の方をカウント
candi = make_divisors(N)[1:]  # 1は考えないでおく
for k in candi:
    n = N
    while n % k == 0:
        n //= k
    if n % k == 1:
        ans += 1
print(ans)
