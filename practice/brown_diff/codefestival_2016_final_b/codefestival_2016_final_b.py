# https://atcoder.jp/contests/cf16-final/tasks/codefestival_2016_final_b

N = int(input())

# [1,N]の部分集合で合計がちょうどNになる.最大値が最小
# 合計が初めてNを超えるときまでの集合を使って作ることができる
# そして差の数字は必ずその集合の中に含まれる

su = 0
for i in range(N + 1):
    su += i
    if su >= N:
        break
# iまでの数字で必ずNは作れる
ans = list(range(1, i + 1))
if su - N != 0:
    ans.remove(su - N)
print(*ans, sep='\n')
