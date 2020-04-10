# https://atcoder.jp/contests/agc003/tasks/agc003_b

enu = enumerate
ra = range
N, *A = map(int, open(0).read().split())
ans = 0


# 先に作れるのは作ったのほうが本当によいのか？←だめ
'''
反例
1...1
2...2
3...1のとき、2同士で先にペアを作る前に上と下に配ったほうがいい
'''
# 1から順にgreedyでは？反例は？なさそう
# ただしカードがないときは作れない(stockが強制的に0になる)ので注意

stock = 0  # or 1
for a in A:
    if a == 0:
        stock = 0
        continue
    a += stock
    ans += a // 2
    stock = a & 1  # %2

print(ans)
