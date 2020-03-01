# https://atcoder.jp/contests/abc050/tasks/arc066_a
# 並び方がありうるかの判別は簡単で、Nが偶数なら、Aがすべて奇数かつすべての数が2回ずつ出現する
# 奇数なら、Aが全て偶数で、0だけ1回出現しほかは2回ずつ出現する。

# ありえるならば、N&1→ 2**(N//2)が答え
# N&1 =0→ 2**(N//2)が答え
# どちらも同じ式でOK
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()

from collections import Counter
A_cnt = Counter(A)


def end():
    print(0)
    exit()


if N & 1:
    if A_cnt[0] != 1:
        end()
    for i in range(2, N, 2):
        if A_cnt[i] != 2:
            end()
else:
    for i in range(1, N, 2):
        if A_cnt[i] != 2:
            end()

print(pow(2, N // 2, 10**9 + 7))
