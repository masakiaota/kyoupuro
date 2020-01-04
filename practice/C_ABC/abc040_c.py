# https://atcoder.jp/contests/abc040/tasks/abc040_c
# やったことあるような気もする
# dpで解ける

# dp[i]をi番目の柱のコストの最小とするとき
# dp[i] = min(dp[i-1]+コスト1,dp[i-2]+コスト2)となる
# ただしコスト1=abs(a[i]-a[i-1])
# ただしコスト2=abs(a[i]-a[i-2])
# np.full と同等のpython実装


def full(shape, full_value):
    if isinstance(shape, tuple):
        sha = shape[::-1]
        ret = [full_value] * sha[0]
        for s in sha[1:]:
            ret = [ret.copy() for i in range(s)]
        return ret
    else:
        return [full_value] * shape


N = int(input())
A = list(map(int, input().split()))

dp = full(N, 10 ** 10)
dp[0] = 0  # 初期化
dp[1] = abs(A[0] - A[1])
for i in range(2, N):
    dp[i] = min(
        dp[i - 1] + abs(A[i] - A[i - 1]),
        dp[i - 2] + abs(A[i] - A[i - 2])
    )

print(dp[-1])
