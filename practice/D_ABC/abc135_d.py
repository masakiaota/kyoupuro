# https://atcoder.jp/contests/abc135/tasks/abc135_d

# む ず す ぎ に き あ く ん 笑

# 全探索すると最悪O(10^100000)となり宇宙が崩壊するので桁の独立性について考えてみる。
# まず文字列Sについて式を書き換える。下からi桁目の数字をSi、len(S)=n (桁数)とすると、
# S=S0 + S1*10 + S2*100 + S3*1000 ... となる。これは(mod 13)において
# S≡S0 + S1*10 + S2*9   + S3*12 ... (mod 13)であり、Si=='?' の中でS≡5となる通りの数を求めれば良い。という問題になる。
# 以上により、i番目の項までの余りはi-1番目の項までの余りを足して法を取り直すことで得られることがわかる。
# 具体的には桁DPなるテクニックで、dp[i][j] (n,13) ... i番目の項まで足したときに13で割った余りがjである通りの数 を更新していけば良い。

MOD = 10**9 + 7
S = input()[::-1]
n = len(S)

dp = [[0] * 13 for _ in range(n + 1)]
dp[0][0] = 1

for i, s in enumerate(S):
    mul = pow(10, i, 13)
    if s != '?':
        shift = (int(s) * mul) % 13
        for j in range(13):
            dp[i + 1][(j + shift) % 13] = dp[i][j]
    else:
        for candi in range(10):
            shift = (candi * mul) % 13
            for j in range(13):
                dp[i + 1][(j + shift) % 13] += dp[i][j] % MOD
print(dp[-1][5] % MOD)
