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


MOD = 10**9 + 7

X, Y = read_ints()
if (X + Y) % 3 != 0 or Y / X > 2 or Y / X < 1 / 2:  # !=0
    print(0)
    exit()

pascal_depth = int((X + Y) / 3)  # パスカルの三角形に当たるn
x = ((X + Y) * 2) // 3
pascal_k = x - X  # 端からいくつずれているか


def combination_mod(n, r, mod=MOD):
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


a = combination_mod(pascal_depth, pascal_k, MOD)
print(a)
