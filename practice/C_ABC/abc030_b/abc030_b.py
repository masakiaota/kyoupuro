# https://atcoder.jp/contests/abc030/tasks/abc030_b
n, m = map(int, input().split())
n = n - 12 if n >= 12 else n
angl = 360 / 60 * m
angs = 360 / 12 * (n + (1 / 60) * m)
if angl < angs:
    angs, angl = angl, angs

print(min(angl - angs, angs + 360 - angl))
