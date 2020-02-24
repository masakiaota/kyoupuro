# https://atcoder.jp/contests/abc147/tasks/abc147_c
# ビット全探索を自作ライブラリでやり直し
# 仮定している不親切な人と、実際に証言で言及されている不親切な人が一致するなかで最大の正直者の場合が答え
# 具体的には、仮定をおく→正直ものの証言を集める→仮定に反する証言がある場合は矛盾
# 矛盾するときは無視、矛盾しないときの人数を記録していく。


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
read = sys.stdin.readline


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


N = read_a_int()
objection = []
for _ in range(N):
    a = read_a_int()
    hatugenn = read_tuple(a)
    objection.append(hatugenn)


def is_ok(katei):
    # 正直もの ==1であるlist kateiから、矛盾するかしないかを返す
    syoujiki = set()  # 仮定した正直者の集合
    for i, k in enumerate(katei):
        if k == 1:
            syoujiki.add(i)
    for s in syoujiki:
        # 発言の収集
        for x, y in objection[s]:
            x -= 1  # 0based-index
            if y == 0:
                if x in syoujiki:
                    return False  # 不親切な人と言われ仮定と矛盾
            else:
                if x not in syoujiki:
                    return False  # 正直者と発言しているが、仮定は正直者ではないとしていて矛盾
    return True


ans = 0
for katei in iter_p_adic(2, N):
    # bit全探索
    if is_ok(katei):
        ans = max(ans, sum(katei))
print(ans)
