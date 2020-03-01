# https://atcoder.jp/contests/abc051/tasks/abc051_c
# 同じ座標を複数回通らないようにということなので、始点と終点の上下左右をすべて使うなかで最短経路を提示すれば良い。
# 一番単純なのは時計回りに動くと決め打ちで文字を出力することかな

sx, sy, tx, ty = map(int, input().split())

dx = tx - sx
dy = ty - sy
U, D, L, R = 'UDLR'

ans = U * dy + R * dx + D * dy + L * dx  # 一周目
ans += L + U * (1 + dy) + R * (1 + dx) + D + R + \
    D * (1 + dy) + L * (1 + dx) + U  # うち間違えに注意
print(ans)
