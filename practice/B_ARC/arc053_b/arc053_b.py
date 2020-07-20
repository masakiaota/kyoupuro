# https://atcoder.jp/contests/arc053/tasks/arc053_b
# 任意に並び替えて回文になる条件→すべての文字数が偶数(文字数が奇数の場合は一つだけ奇数を許容する)
# 問題となるのは常に奇数の文字
# 奇数の個数が重要そうだけどわからんなぁ
# 奇数の個数個に分割する必要がありそう
S = input()
from collections import Counter
cnt = Counter(S)
n_odd = 0
for k, v in cnt.items():
    n_odd += v & 1
if n_odd == 0:
    print(len(S))
    exit()

ans = len(S) // n_odd
print(ans if ans & 1 else ans - 1)
