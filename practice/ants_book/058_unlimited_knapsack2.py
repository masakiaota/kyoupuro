# コイン問題の応用で解けるので解いてみる
n = 3
W = [3, 4, 2]
V = [4, 5, 3]
W_max = 7

'''
dp[j] ... ピッタリ重さがjになる中で最大の価値を記録する
更新
dp[j+W[i]] = max(dp[j+W[i]], dp[j]+V[i])
'''

dp = [-1] * (W_max + 1)  # -1は作れないことを意味する
dp[0] = 0  # 重さがピッタリ0になるときは価値が0
for j in range(W_max):
    if dp[j] == -1:  # これが作れないということはここから先ちょうどの遷移は不可能
        continue
    for w, v in zip(W, V):
        if j + w > W_max:
            continue
        dp[j + w] = max(dp[j + w], dp[j] + v)

print(max(dp))  # W_max以下で最大の価値が抽出できる。
