N = 4
K = 11
L = [8.02, 7.43, 4.57, 5.39]

# 決め打ち二分探索 (めぐる式)
# K本以上作れる長さxの内、最大のxを探索する


def is_ok(x):
    n = 0  # 何本長さxの紐が作れるか
    for l in L:
        n += l // x
    return n >= K


def meguru_bisect(ng, ok):
    '''
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while abs(ok - ng) > 10 ** -3:  # 10^-3の誤差は許容される
        mid = (ok + ng) / 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(10**6, 0.1))  # 二桁を出力しろだが本質ではないので全部出力しちゃう
