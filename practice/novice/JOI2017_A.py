# 問題　https://atcoder.jp/contests/joi2008yo/tasks/joi2008yo_a

target = 1000 - int(input())

coinlist = [500, 100, 50, 10, 5, 1]

ans = 0
for c in coinlist:
    if target // c:
        # より大きいコインで払えるなら払う
        ans += target // c
        target = target % c

print(ans)
