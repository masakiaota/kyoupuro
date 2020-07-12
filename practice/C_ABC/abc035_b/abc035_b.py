# https://atcoder.jp/contests/abc035/tasks/abc035_b
from collections import Counter
S = input()
T = int(input())
cnt = Counter(S)
x = cnt['R'] - cnt['L']
y = cnt['U'] - cnt['D']
if T == 1:
    # 最大は簡単
    print(abs(x) + abs(y) + cnt['?'])
else:
    # 最小はどうしようか
    n = cnt['?']  # 自由に動ける回数
    ans = abs(x) + abs(y) - n
    print(ans if ans > 0 else ans % 2)
