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

# 条件にあうやつを数え上げる。
# 条件に合えば、10の倍数は基本的に条件を満たすが
from collections import defaultdict
container = defaultdict(lambda: 0)

for n in range(N + 1):
    if n % 10 == 0:
        continue
    n_str = str(n)
    container[(int(n_str[0]), int(n_str[-1]))] += 1

ans = 0
from itertools import product
for i, j in product(range(1, 10), range(1, 10)):
    ans += container[(i, j)] * container[(j, i)]

print(ans)
