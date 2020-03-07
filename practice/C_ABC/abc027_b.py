# https://atcoder.jp/contests/abc027/tasks/abc027_b
# 人数を島の数で割り切れない場合は-1
# 一致したら,人数//島の数(n_person) となるような操作をしたい
# 具体的には、greedyに前から島の人数を足していったとき、n_person*見た島の数に数字が等しければ、その区間には橋をかけたい
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


N = int(input())
A = read_ints()
sumA = sum(A)
if sumA % N:
    print(-1)
    exit()

n_person = sumA // N
n_disjoint = 0
tmp = 0
cnt = 0
for a in A:
    tmp += a
    cnt += 1
    if tmp == n_person * cnt:
        # そこまでに橋が連続して必要
        # →次の橋は必要ない
        n_disjoint += 1
        tmp = 0
        cnt = 0
print(N - n_disjoint)
