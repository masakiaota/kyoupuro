# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


n, k = read_ints()
# dpっぽい
# i回目の通りの数はi+1回目の通りの数にどのような影響を及ぼすだろうか？
# でもkが10**9なのでこれでdpするのは間に合わない。
# nの向きでdpか？
# もし部屋が1個あったとき、k回1人移動したときに何通り？→1通り
# 部屋が2個あったときは？k=1のときは2通り、k>1のときは3通り


# nが3以上のとき、kがnより大きければ任意の組み合わせができるのでは？
# ならばkでdpしても問題ない だけど うまく作れない

# kを大きくするごとに0を多くできる

# わかった！
# 0が1つ入ったとき、2つ入ったときと他の数字の並び変えた通りの数を合計すりゃいいのはわかるんだけど
# 通りの数がわからん？
