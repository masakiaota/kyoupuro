# https://atcoder.jp/contests/arc054/tasks/arc054_a
L, X, Y, S, D = map(int, input().split())
if S - D >= 0:
    d_back = S - D
    d_front = L - d_back
else:
    d_front = D - S
    d_back = L - d_front

print(min(d_front / (X + Y), (d_back / (Y - X)) if (Y - X) > 0 else 2**31))
