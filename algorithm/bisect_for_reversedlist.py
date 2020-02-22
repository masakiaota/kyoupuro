# .sort(reverse=True)にしたリストに対してもbisectできるようにしたい

a = [10, 9, 5, 3, 3, 3, 2, -3, -3]


def is_ok(arg):
    # 条件を満たすかどうか？問題ごとに定義
    pass


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


#     0  1  2  3  4  5  6   7   8
a = [10, 9, 5, 3, 3, 3, 2, -3, -3]


from bisect import bisect_left


def bisect_left_reverse(a, x):
    '''
    reverseにソートされたlist aに対してxを挿入できるidxを返す。
    xが存在する場合には一番左側のidxとなる。
    '''
    if a[0] <= x:
        return 0
    if x < a[-1]:
        return len(a)

    # 二分探索
    ok = len(a) - 1
    ng = 0
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if a[mid] <= x:
            ok = mid
        else:
            ng = mid
    return ok


print(bisect_left_reverse(a, 4))
print(bisect_left_reverse(a, 9))
print(bisect_left_reverse(a, 10))
print(bisect_left_reverse(a, 3))
print(bisect_left_reverse(a, -3))
print(bisect_left_reverse(a, -4))


def bisect_right_reverse():
    pass
