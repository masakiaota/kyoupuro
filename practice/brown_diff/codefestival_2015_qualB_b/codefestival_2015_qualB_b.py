# https://atcoder.jp/contests/code-festival-2015-qualb/tasks/codefestival_2015_qualB_b
from collections import Counter
N, M, *A = open(0).read().split()
a, b = Counter(A).most_common()[0]
print(a if b > int(N) // 2 else '?')
