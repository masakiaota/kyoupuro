# https://atcoder.jp/contests/abc135/tasks/abc135_c

N = int(input())
A = list(map(int, input().split()))
B = list(map(int, input().split()))
ans = 0
l = A[0]
for i in range(N):
    r = A[i + 1]
    cur = B[i]
    # print(l, r, cur)
    # ans += min(l + r, cur)
    # l = max(r - max(cur - l, 0), 0)  # めちゃくちゃ整理するとこの形の式になる

    # わかりやすくかきなおす
    ans += min(l, cur)
    rest_power = max(cur - l, 0)
    ans += min(rest_power, r)
    l = max(r - rest_power, 0)  # next l

print(ans)
