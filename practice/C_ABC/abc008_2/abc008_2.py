# https://atcoder.jp/contests/abc008/tasks/abc008_2
N = int(input())
S = [input() for _ in range(N)]
from collections import Counter
print(Counter(S).most_common()[0][0])
