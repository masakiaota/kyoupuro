# https://atcoder.jp/contests/agc038/tasks/agc038_a

# 条件を満たすものは必ず存在するのか？ 制約により存在しそう。→かならず存在する
# 条件を満たすにはどうならべたらいいのか？
# →例えば以下のようにすれば良い
# 条件を満たす行を設定する。この行をB個繰り返す。行の01を反転させて残りを埋める(片方を固定したらもう片方は独立に決定できるイメージ)

H, W, A, B = map(int, input().split())
base = '0' * A + '1' * (W - A)
fliped = '1' * A + '0' * (W - A)

for _ in range(B):
    print(base)
for _ in range(H - B):
    print(fliped)
