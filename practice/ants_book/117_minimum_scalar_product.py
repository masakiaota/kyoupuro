# 大小関係を逆にソートする
# 逆にソートされた状態から適当なi,jをひっくり返すと内積が大きくなることが示せる。よって最適な状態であると言える。


def solve(v1, v2):
    v1.sort()
    v2.sort(reverse=True)
    return sum([x * y for x, y in zip(v1, v2)])


# 入力例1
n = 3
v1 = [1, 3, -5]
v2 = [-2, 4, 1]
print(solve(v1, v2))

# 入力例2
n = 5
v1 = [1, 2, 3, 4, 5]
v2 = [1, 0, 1, 0, 1]
print(solve(v1, v2))
