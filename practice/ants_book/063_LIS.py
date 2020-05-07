from bisect import bisect_left, bisect_right

# 真に増加する部分列の最長を知りたい
n = 5
A = [4, 2, 3, 1, 5]

# 蟻本とは異なり、長さを可変にしておく(省メモリだしlen(dp)をするだけでLISが取得可能)
dp = []
for a in A:
    print(dp)
    idx = bisect_right(dp, a)  # 初めて真に大きい要素になるidx
    if idx == len(dp):
        dp.append(a)
    else:
        dp[idx] = a  # aに更新
print(dp)
print(len(dp))
