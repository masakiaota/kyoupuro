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


# 偶数番目に0の個数を格納するようなデータ構造を構築する #連長圧縮
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

# しゃくとり法で実装
add = min(2*K+1, len(num_cnts))  # Kで指定するよりも短い場合はそっちに合わせる
ans = 1

# forの外にleft, rightを持つ
left = 0
right = 0
tmp = 0  # num_cnt[left:right]のsum

# しゃくとりしながら最大長の探索
for next_left in range(0, len(num_cnts) - add + 1, 2):
    # 次のleft,rightを計算する #今回はあまりしゃくとりのありがたみがない
    next_left = next_left  # あえてこう書いてる
    next_right = next_left+add

    # 左端を移動する (左側の移動によって減る分を計算する)
    while (next_left > left):
        tmp -= num_cnts[left]
        left += 1

    # 右端を移動する
    while (next_right > right):
        tmp += num_cnts[right]
        right += 1

    ans = max(ans, tmp)

print(ans)
