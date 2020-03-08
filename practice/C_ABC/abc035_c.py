# https://atcoder.jp/contests/abc035/tasks/abc035_c
# これも水色パフォ
# 重要な性質 ... 操作の順番は関係ない。→拡張して、rとlの操作も一致しなくて良い
# →結局ごっちゃにして各idxについてcnt、 &1ならflipする
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


from collections import Counter
N, Q = read_ints()
ls = []
for _ in range(Q):
    l, r = read_ints()
    ls.append(l)
    ls.append(r + 1)  # 半開区間で扱うために
cnt = Counter(ls)

ans = ''
now = 0
for i in range(1, N + 1):
    if cnt[i] & 1:
        now = 1 - now

    if now == 1:
        ans += '1'
    else:
        ans += '0'

print(ans)
