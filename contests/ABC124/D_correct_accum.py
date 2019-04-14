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
    return [read() for _ in range(H)]


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


N, K = read_ints()
S = read()[:-1]


# 偶数番目に0の個数を格納するようなデータ構造を構築する
num_cnts = []
now = '1'  # 0から考慮する
cnt = 0
for s in S:
    if s == now:
        cnt += 1
    if s != now:  # もしいまカウントしている文字と異なったらカウントを格納して、カウントをリセットする。
        num_cnts.append(cnt)
        now = s
        cnt = 1
num_cnts.append(cnt)
# もしSが0で終わっていたら0個の1を付け足す
if S[-1] == '0':
    num_cnts.append(0)

assert len(num_cnts) % 2 == 1, '奇数個じゃない！'

# 累積和で実装
from itertools import accumulate
num_accums = [0]+list(accumulate(num_cnts))

# 最大長の探索
add = min(2*K+1, len(num_cnts))  # Kで指定するよりも短い場合はそっちに合わせる
ans = 1

# print(num_cnts)
for left in range(0, len(num_cnts) - add+1, 2):
    right = left + add
    ans = max(ans, num_accums[right] - num_accums[left])
    # print(left, right, ans)

print(ans)
