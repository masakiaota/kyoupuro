# https://atcoder.jp/contests/abc127/tasks/abc127_d
# 難しいこと考えすぎないこと。数字を書き換えるとは言ってるけどCjをBj枚追加して大きい方からN枚取るのが最適
# 具体的にBj枚追加していくと死ぬので、枚数はタプルでその数とともに保持しておく


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


from collections import Counter

N, M = read_ints()
A = read_ints()
BC = read_tuple(M)


cnter = Counter(A)
for b, c in BC:
    cnter[c] += b

num_and_cnt = list(cnter.items())
num_and_cnt.sort(reverse=True)

ans = 0
total = 0
for num, cnt in num_and_cnt:
    if total + cnt <= N:
        total += cnt
        ans += num * cnt
    else:
        while total < N:
            ans += num
            total += 1

print(ans)
