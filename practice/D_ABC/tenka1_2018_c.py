# https://atcoder.jp/contests/tenka1-2018-beginner/tasks/tenka1_2018_c
# 直感的には大きい小さいを交互に並べれば良さそうだが...?

# Nが偶数なら交互でok
# Nが奇数なら、短い方を真ん中に入れれば良い

# 1 5 7 10
# 1 10 5 7 →16# 最後に行くに連れ差は少なくなってく
# 1 7 5 10 →13# 差がマイルドになるのでは?

# ソートしたとき一番最初と一番最後は必ず差が最大。
# それ以外の選び方をした場合差はそれより必ず小さくなるので、ソートの端から選んでいくのが最適だとわかる。
# しかし最後に関しては差が小さく微妙。実は中央値を先頭に付け足したほうが良い。

# あってはいたけど上記の考察はまだまだ甘い、>= <= >=などの場合と偶数奇数の場合を分けて定式化して
# 考察すればもっとわかったはず


N = int(input())
A = []
for _ in range(N):
    A.append(int(input()))

A.sort()

# もしNが偶数のとき
if N & 1 == 0:
    now = A[N // 2]
    ans = 0
    for i in range(N // 2):
        for idx in [i, -(i + 1)]:
            if idx == -N // 2:
                break
            ans += abs(now - A[idx])
            now = A[idx]
    print(ans)
else:  # Nが奇数のとき
    now = A[N // 2]
    ans1 = 0
    for i in range(N // 2):
        for idx in [i, -(i + 1)]:
            ans1 += abs(now - A[idx])
            now = A[idx]

    now = A[N // 2]
    ans2 = 0
    for i in range(N // 2):
        for idx in [-(i + 1), i]:
            ans2 += abs(now - A[idx])
            now = A[idx]
    print(max(ans1, ans2))
