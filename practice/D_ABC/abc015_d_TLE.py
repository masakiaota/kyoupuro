def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数リストを次々返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


from itertools import product
W = read_a_int()
N, K = read_ints()
AB = read_tuple(N)

ans = 0
for p in iter_p_adic(2, N):
    if sum(p) > K:
        continue
    w = 0
    v = 0
    for i, pp in enumerate(p):
        if pp == 0:
            continue
        a, b = AB[i]
        w += a
        v += b
        if w > W:
            v = 0
            break
    ans = max(ans, v)
print(ans)
