# https://atcoder.jp/contests/abc044/tasks/arc060_a
# ソートしてからごちゃごちゃやればどうにかなりそう(無理でした)

'''
解説満点解法1
もしもk枚選んだとき、合計がsになる通りの数がわかったら？→s=k*Aとなる場合の数の合計をすれば良い

k枚選んだとき、合計がsになる通りの数は典型的なDP！
dp(j,k,s)... X[0:j]からk枚選んで、その合計をsにするような選び方の総数
と定義すると、

更新則
dp(j+1,k,s) = X[j]を選んだ結果sになったときの通りの数 + X[j]を選ばずにsになっている通りの数 なので
すなわち
dp(j+1,k,s) = dp(j,k-1,s-X[j]) + dp(j,k,s)

あとは初期条件と境界条件と伝播条件を付け加える。
境界条件 dp(j,k,0)=0 (数枚選んで合計が0になるのはありえない)
初期条件 dp(j,0,0)=1 (ただし、0枚選んだときは1通りとなる)(s-X[j]=0のときに1通りとなってほしいという気持ちもある)
伝播条件 s-X[j]<0に関しては dp(j,k-1,s-X[j])が必ず0通りとなる(ありえないので)
'''

import sys
read = sys.stdin.readline
from itertools import product


def read_ints():
    return list(map(int, read().split()))


N, A = read_ints()
X = read_ints()
S = sum(X)

# 満点解法1
dp = [[[0] * (S + 1) for _ in range(N + 1)] for _ in range(N + 1)]
for j in range(N + 1):  # 初期条件
    dp[j][0][0] = 1  # 1枚も選ばずに0になるのは1通り
# 0,0,0は1でいい∵ j,1,xで1通りになりたいときに1だと都合がいい

for j, k, s in product(range(N), range(1, N + 1), range(S + 1)):
    dp[j + 1][k][s] = dp[j][k][s] + \
        (0 if s - X[j] < 0 else dp[j][k - 1][s - X[j]])

# s=k*Aとなるような通りの数の総数
ans = 0
for k in range(1, N + 1):  # 0個はとる通りの数はいらない
    s = k * A
    if s <= S:
        ans += dp[N][k][s]

# print(dp)
print(ans)


'''
満点解法2
平均値の特性からXから差っ引けば、選んだカードの合計が0になれば良くなる。Y=X-Aと定義する。
→そしたらYからいくつか選んで合計tをつくる総数をdpで求めれば良い。(負の価値もありえるナップザックDPの通りの数版)

0枚以上選んだときに合計がtに成るかどうか考える

dp(j,t)...Y[:j]の中から0枚以上選んで、その合計を(t-NX)(負になる可能性もあるだけ)にするような選び方の総数
dp(j,t+NX)...Y[:j]の中から0枚以上選んで、その合計をtにするような選び方の総数 この定義のほうがいいかな

更新則
dp(j+1,t+NX) = Y[j]を選んで合計がtになる総和 + Y[j]を選ばずに合計がtになっている総和
つまり
dp(j+1,t+NX) = dp(j,t+NX - Y[j]) + dp(j,t+NX)

境界条件
dp(0,t+NX)=0  一つも考慮しない状態では必ずスコアは作れない
初期条件
dp(0,0+NX)=1 ∵ただし一つも考慮しないならば合計が0になるのは当然1通り
伝播条件
0<=t+NX-Y[j]<=2NXとなる範囲外の場合はdp(j,t+NX - Y[j])を無視 
'''

# 満点解法2
# NXは十分幅を取れていればなんでもいい
# NX = N * max(X) #今回は最低限の幅Sで代用
S = sum(X)

Y = []
for x in X:
    Y.append(x - A)
dp = [[0] * (2 * S + 1) for _ in range(N + 1)]

# 初期条件
dp[0][S] = 1

for j, t in product(range(N), range(-S, S + 1)):
    ts = t + S
    dp[j + 1][ts] = dp[j][ts]
    dp[j + 1][ts] += dp[j][ts - Y[j]] if 0 <= ts - Y[j] <= 2 * S else 0

print(dp[N][S] - 1)
