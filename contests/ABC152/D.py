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


N = read_a_int()
N_str = str(N)
n_keta = len(N_str)

if N < 10:
    print(N)
    exit()

# ここでは10以上の場合
ans = 9
for n in range(2, n_keta):
    for nn in range(1, n):
        # 今考えている桁数まで
        ans += 81 * (10 ** (n - 2)) * 81 * (10 ** (nn))
    ans += 2 * 81 * (10**(n - 2))

# ここで最後にいくつタスカ全探索
# 最上位の数未満の通りの数は確定可能
# saijouimiman = int(N_str[0]) - 1
# ans += (saijouimiman ** 2) * (10 ** (n_keta - 2))

# 最上位時に満たす数の通りは？
# saijoui = int(N_str[0])
# for i in range(10 ** (n_keta - 1)):
#     now = saijoui


print(ans)
