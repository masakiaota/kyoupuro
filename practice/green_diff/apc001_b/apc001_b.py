# https://atcoder.jp/contests/apc001/tasks/apc001_b


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
rr = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
B = read_ints()

'''
aiとbiについて
ai<=biのとき→必ず等しくできる
ai>biのとき→ai-biをbiに加算してやっと等しくなれる。#Aのどこかに余分な2*(ai-bi)を加算しなくてはいけない(バラけてもokだけど)
余分な分を吸収してくれるような数字を探したい

→
まずai>biについてをai-biの合計n1を計算する(1を足した回数。これと同じ数2をAに足さないと行けない)
次に、ai+2<=biの各要素についてbi-ai(>=2)を計算し、n1を0に消費できるか計算する。
消費できればYes、できなればNo
(n1を0にした時点では数列は等しくなっていないが、ai<biについて1回以上の操作を追加することで必ず等しくできることが保証できる)
'''

n1 = 0  # bに+1する回数
for a, b in zip(A, B):
    if a > b:
        n1 += a - b

for a, b in zip(A, B):
    if a + 2 <= b:
        n1 -= (b - a) // 2


print("Yes" if n1 <= 0 else "No")
