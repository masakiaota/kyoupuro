# https://atcoder.jp/contests/agc008/tasks/agc008_a
# どう見ても符号反転は1回するのが最適
# ひたすら場合分け


x, y = map(int, input().split())
if x * y < 0:  # 異符号ならば答えは一つ
    print(abs(abs(x) - abs(y)) + 1)
elif 0 <= x < y:
    print(y - x)
elif 0 < y < x:
    print(2 + x - y)
elif y == 0 and y < x:
    print(1 + x)
elif x < y <= 0:
    print(y - x)
elif y < x < 0:
    print(2 + x - y)
elif x == 0 and y < x:
    print(1 - y)
