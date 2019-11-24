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

# 戦略
# 一番ハブの多いやつをrootにした木で幅優先する?いや1でいい
# 3つのノードがまっすぐ連なるところは過去の色を再利用したい
# ハブになっているノードが問題では？


# やっぱり幅優先

# どうやって最後に並び替える？そもそも並び替える必要がないか
# color = [1]
# ans = []
# cur_color = 0
# pre_color = 0
# defaultcolor = 1
# pre_parent = 0
# for _ in range(N):
#     a, b = read_ints()
#     if a != pre_parent:
#         # 新しく色付けを始める
#         cur_color = defaultcolor
#         ans.append(cur_color)
#     elif a == pre_parent:
#         # 色付けを拡張していく
#         cur_color += 1
#         ans.append(cur_color)


from collections import defaultdict, deque
tree = defaultdict(lambda: [])
inputls = []
ans = dict()
for _ in range(N - 1):
    a, b = read_ints()
    inputls.append((a, b))
    tree[a].append(b)  # 一方方向だけでいいよね
    ans[(a, b)] = 0  # 初期化

# bfs
que = deque([(1, 0)])  # (親ノード,親の前の色情報)
defcolor = 1
while que:
    cur, pre_color = que.popleft()
    cur_color = defcolor
    for nx in tree[cur]:
        if cur_color == pre_color:
            cur_color += 1

        que.append((nx, cur_color))
        ans[(cur, nx)] = cur_color
        cur_color += 1


print(max(ans.values()))
for a, b in inputls:
    print(ans[(a, b)])
