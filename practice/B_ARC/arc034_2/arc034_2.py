# https://atcoder.jp/contests/arc034/tasks/arc034_2
# f(x)<=17*9なので探索するxの範囲はだいぶしぼれる
N = int(input())
ans = []
for x in range(max(1, N - 17 * 9), N + 1):
    if x + sum(map(int, str(x))) == N:
        ans.append(x)
print(len(ans))
if len(ans):
    print(*ans, sep='\n')
