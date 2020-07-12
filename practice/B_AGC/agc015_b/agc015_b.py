# https://atcoder.jp/contests/agc015/tasks/agc015_b
# 自分から見て直接行ける方向は1回、それ以外は2回ですむ
S = input()
N = len(S) - 1
ans = 0
for i, s in enumerate(S):
    n_once = i if s == 'D' else N - i
    ans += n_once + 2 * (N - n_once)
print(ans)
