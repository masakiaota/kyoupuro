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


n = 0
N = read_a_int()
A = []
for a in read().split():
    A.append(int(a))
    n = max(n, len(bin(int(a))) - 2)


def ret_binary(a: int):  # 逆順で返すことに注意
    tmp = bin(a)[2:]
    ret_r = [0] * n
    for i, t in enumerate(tmp[::-1]):
        ret_r[i] = int(t)
    return ret_r


def calc_sum(a_binary, n_1_ls, n_1_max):
    '''
    a_binaryはxorしてから和を取りたい数
    n_1_lsは各桁の1の数
    n_1_maxは最大1は何個あり得るか
    '''
    ret = 0
    for i, (aa, n_1) in enumerate(zip(a_binary, n_1_ls)):
        if aa == 0:
            ret += (((2**i) % MOD) * n_1) % MOD
        else:
            ret += (((2**i) % MOD) * (n_1_max - n_1)) % MOD
    return ret


def add_koko(lsa, lsb):
    return [a + b for a, b in zip(lsa, lsb)]


MOD = 10**9 + 7

n_1_max = 0
n_1_ls = [0] * n
ans = 0
a_pre = A[N - 1]
a_pre_binary = ret_binary(a_pre)
for a in A[-2::-1]:
    a_binary = ret_binary(a)
    n_1_ls = add_koko(n_1_ls, a_pre_binary)
    n_1_max += 1
    tmp = calc_sum(a_binary, n_1_ls, n_1_max)
    ans += tmp
    ans %= MOD
    a_pre_binary = a_binary

print(ans)
