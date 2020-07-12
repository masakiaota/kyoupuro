# https://atcoder.jp/contests/agc029/tasks/agc029_a
# Wをなるべく右に詰めようとしたときに移動する距離の総和が答え
ans = 0
cnt = 0  # Wの移動先
for i, s in enumerate(input()):
    if s == 'W':
        ans += i - cnt
        cnt += 1
print(ans)
