# 牛の座標をciとするとmax_{配置} min_i (c_{i+1}-c_i) を求める問題
# 牛を配置したときに最小の距離がmin_i (c_{i+1}-c_i)がdであるとしたときに、矛盾することなく牛を並べることができる最大のd
# と問題を言い換えられる。
N = 5
M = 3
X = [1, 2, 8, 4, 9]


X.sort()


def is_ok(d):
    # 間隔d以上で牛を並べることができればTrue
    nex = -10000
    n = 0  # 並んだ牛の数
    for x in X:
        if x >= nex:
            n += 1
            nex = x + d
    return n >= M


def meguru_bisect(ng, ok):
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(10**9 + 1, -1))
