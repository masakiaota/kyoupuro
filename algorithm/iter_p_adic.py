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


def g_idx(perm):  # 生成されたリストから1が立つもののidxだけ返す関数 (bit全探索用)
    candi = []
    for i, p in enumerate(perm):
        if p == 0:
            continue
        candi.append(i)
    return candi


# test
iterator = iter_p_adic(4, 3)
for idxs in iterator:
    print(idxs)
