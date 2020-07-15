# https://atcoder.jp/contests/code-festival-2015-qualb/tasks/codefestival_2015_qualB_c
# greedyにやっていけば良さそう
N, M = map(int, input().split())
A = sorted(map(int, input().split()))
B = sorted(map(int, input().split()))
while B:
    b, a = B.pop(), A.pop()
    if a < b or M > N:
        print('NO')
        exit()
print('YES')
