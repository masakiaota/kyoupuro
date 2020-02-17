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


from functools import lru_cache
import sys
sys.setrecursionlimit(1 << 25)

N = read_a_int()
siharai = ''


def F(n):
    '''
    支払う金額を返す。再帰関数
    '''
    global siharai
    # 終了条件
    assert n > -1
    if n < 5:
        siharai += str(n)
        return 0

    # 桁dpもどき
    # 1の位が4以下ならそのまま紙幣で支払うのが得→自身をそのまま答えに採用
    # 6以上ならばお釣り狙いのほうが得→自身は0にセットして、上位桁を+1する
    # 5ちょうどのときは？ #お釣りもそのまま払っても同じ。だけど上位 お釣り狙いだと上の桁が多くなって支払う金額が増える可能性がある？
    # 5ピッタリのときに上位桁に+1すると悪くなる場合が存在する 455 とは 15にってしまう
    # 5ピッタリのときはどうするのがよいのか？
    q, r = divmod(n, 10)
    if r == 5:
        qq, rr = divmod(q, 10)
        if rr < 5:
            # そのまま
            joint = 5
            F(q)
        else:
            joint = 0
            F(q + 1)
            # +1
    elif r < 5:
        joint = r
        F(q)
    else:
        joint = 0
        F(q + 1)
    siharai += str(joint)


F(N)
siharai = int(siharai)
oturi = siharai - N
# print(siharai, oturi)

ans = 0
for a in str(siharai) + str(oturi):
    ans += int(a)

# print(sum(list(siharai)) + sum(list(str(oturi))))
# print(sum(list(siharai)))
print(ans)
