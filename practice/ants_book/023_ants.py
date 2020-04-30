# 入力例
L = 10
n = 3
x = [2, 6, 7]


# 最小の時間も最大の時間も一番端のもの
ans_min = min(L - x[-1], x[0])
ans_max = max(x[-1], L - x[0])
print(ans_min, ans_max)
