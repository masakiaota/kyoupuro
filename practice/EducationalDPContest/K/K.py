# https://atcoder.jp/contests/abc176/tasks
'''
dp[i]...次に行動するプレイヤーが勝つかどうか。石がi個あったときに。

dp[i]=True if (i-a for a \in A で一つでもFalseがあったら) else False
これは配るdpを用いて
dp[i+a] |= 1-dp[i] で書くことができる
'''

N, K = map(int, input().split())
A = list(map(int, input().split()))
dp = [0] * (K + max(A) + 1)
for i in range(K):
    for a in A:
        dp[i + a] |= 1 - dp[i]
print('First' if dp[K] else 'Second')
