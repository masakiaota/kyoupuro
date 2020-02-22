# .sort(reverse=True)にしたリストに対してもbisectできるようにしたい

#     0  1  2  3  4  5  6   7   8
a = [10, 9, 5, 3, 3, 3, 2, -3, -3]


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


print(a)
test = [4, 9, 11, 10, 3, -3, -4]
for t in test:
    print(t, bisect_left_reverse(a, t))


#     0  1  2  3  4  5  6   7   8
a = [10, 9, 5, 3, 3, 3, 2, -3, -3]


def bisect_right_reverse(a, x):
    '''
    reverseにソートされたlist aに対してxを挿入できるidxを返す。
    xが存在する場合には一番右側のidx+1となる。
    '''
    if a[0] < x:
        return 0
    if x <= a[-1]:
        return len(a)
    # 二分探索
    ok = len(a) - 1
    ng = 0
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if a[mid] < x:
            ok = mid
        else:
            ng = mid
    return ok


print(a)
test = [4, 9, 11, 10, 3, -3, -4]
for t in test:
    print(t, bisect_right_reverse(a, t))
