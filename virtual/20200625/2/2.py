A, B = map(int, input().split())
# https://atcoder.jp/contests/abc090/tasks/abc090_b
# 全探索でよくね？


def is_cycle(x):
    x = str(x)
    return x == x[::-1]


ans = 0
for x in range(A, B + 1):
    ans += is_cycle(x)
print(ans)
