# max_{i=i_1,i_2,...,i_k} (Σvi/Σwi) となるようなiの選び方
# これも答えをxと仮定すると、x以上となる選び方が存在する中の最大のx
# Σ(vi-x*wi) >= 0 となるので greedyにk個選んだときに条件を満たすことができるか判別できる
# 計算量は O(NlogNlog(max x))


N = 3
K = 2
W = [2, 5, 2]
V = [2, 3, 1]


def is_ok(x):
    # 単位価値がx以上となる選び方は存在するか？
    VXW = [vi - x * wi for vi, wi in zip(V, W)]
    VXW.sort(reverse=True)
    return sum(VXW[:K]) >= 0


def meguru_bisect(ng, ok):
    while (abs(ok - ng) > 10**-9):  # 小数8桁ぐらいの精度は保証する
        mid = (ok + ng) / 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(10**6 + 1, 0))
