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


n = 60
N = read_a_int()
A = read_ints()


def ret_binary(a: int):  # 逆順で返すことに注意
    ret_r = [0] * n
    for i in range(n):
        bit = a % 2
        a = a // 2
        ret_r[i] = bit
    return ret_r


MOD = 10**9 + 7

n_1_max = 0
n_1_ls = [0] * n
ans = 0
for a in A:
    tmp = ret_binary(a)
    for i in range(n):
        n_1_ls[i] += tmp[i]


for i, n_1 in enumerate(n_1_ls):
    num_sonoketa = n_1 * (N - n_1)
    ans += (((2**i) % MOD) * num_sonoketa)
    ans %= MOD
print(ans)
