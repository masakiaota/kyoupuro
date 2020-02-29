# https://atcoder.jp/contests/abc102/tasks/arc100_a
# Aの数列を前処理で加工し、順番に依存しない数列Bに加工する
# abs(Bi-x)の和を最小化するxはBの中央値である。(立式しxで微分したのちKについて増減表を書いてみよ)
# それを答えとすれば良い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


N = read_a_int()
A = read_ints()
B = [a - i for a, i in zip(A, range(1, N + 1))]
B.sort()

x = B[N // 2]
ans = 0
for b in B:
    ans += abs(x - b)


print(ans)

'''
以下式をごちゃごちゃやったときの回答
'''


class cumsum1d:
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


# N = read_a_int()
# A = read_ints()
if N == 1:
    print(0)
    exit()
B = [a - i for a, i in zip(A, range(1, N + 1))]
B.sort()

# 絶対値を区切る境目はどこがよいか？
B_cum = cumsum1d(B)
ans = 10**15
for k in range(1, N):
    tmp_sum = -B_cum.total(0, k) + B_cum.total(k, N)
    add = min((2 * k - N) * B[k - 1], (2 * k - N) * B[k])
    ans = min(ans, tmp_sum + add)


print(ans)
# 式ごちゃごちゃやったけど絶対楽な方法がある。
# 明日復習
