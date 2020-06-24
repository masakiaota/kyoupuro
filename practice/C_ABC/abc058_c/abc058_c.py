# https://atcoder.jp/contests/abc058/tasks/arc071_a
# 共通要素を抜き出しソート

from collections import Counter, defaultdict
from functools import reduce
n = int(input())
S = []
for _ in range(n):
    S.append(Counter(input()))


def f(x, y):
    '''各文字数のminをとる、最大公約数的なね'''
    ret = defaultdict(lambda: 0)
    for key in set(list(x.keys()) + list(y.keys())):
        ret[key] = min(x[key], y[key])
    return ret


res = reduce(f, S)
ans = []
for key in sorted(res.keys()):
    ans.append(key * res[key])
print(''.join(ans))
