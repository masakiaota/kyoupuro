# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/7/ALDS1_7_B

import sys
sys.setrecursionlimit(2**20)  # 再帰回数上限の向上 かなり多くしないとREになる


class Node:
    def __init__(self,
                 parent,
                 left,
                 right):
        self.parent = parent
        self.left = left  # 左の子ノード
        self.right = right  # 右の**子**


def get_all_height(T: dict, H: dict, u: int):
    '''
    深さ優先探索で各地点をめぐりながらHに高さ情報セットしていく
    '''
    h_left, h_right = 0, 0  # Noneによって代入されなかったとき用
    # 本では右から左の方に探索していったけど
    # キモいのでここでは左から右に探索していくことにする
    if T[u].left is not None:
        h_left = get_all_height(T, H, T[u].left) + 1
    if T[u].right is not None:
        h_right = get_all_height(T, H, T[u].right) + 1
    ret = max(h_left, h_right)
    H[u] = ret
    return ret


def get_all_depth(T, D: dict, u, p: int):
    '''
    まあHがわかってればdepthに変換可能なんだけど練習のために再帰関数で実装する。

    Dは深さを格納するための辞書
    pは現在の深さ
    '''
    D[u] = p
    if T[u].left is not None:
        get_all_depth(T, D, T[u].left, p+1)
    if T[u].right is not None:
        get_all_depth(T, D, T[u].right, p+1)


def ret_sibling(T, u):
    '''
    一度親を経由すれば良い
    ココらへんの実装が汚くなるからNILを-1にしておけば良かったなぁと後悔
    '''
    parent = T[u].parent
    if parent is None:
        return -1  # 根は兄弟なし
    ret = T[parent].left if T[parent].left != u else T[parent].right
    if ret is None:
        return -1
    return ret


def ret_degree(T, u):
    ret = 2
    if T[u].left is None:
        ret -= 1
    if T[u].right is None:
        ret -= 1
    return ret


# データを読み込む
n = int(input())
T = {key: Node(None, None, None) for key in range(n)}  # 初期化
for _ in range(n):
    tmp = list(map(int, input().split()))
    # 左から処理
    if tmp[1] != -1:
        T[tmp[0]].left = tmp[1]  # 子の代入
        T[tmp[1]].parent = tmp[0]  # 親の代入
    if tmp[2] != -1:
        T[tmp[0]].right = tmp[2]  # 子の代入
        T[tmp[2]].parent = tmp[0]  # 親の代入

# 根の探索
for k, v in T.items():
    if v.parent is None:
        ROOT = k
        break
else:
    raise ValueError("ROOTが存在しないなんておかしい")

# 深さと高さの探索
D = {}
get_all_depth(T, D, ROOT, 0)
H = {}
get_all_height(T, H, ROOT)


def print_for_a_node(u):
    if T[u].parent is None:
        parent = -1
    else:
        parent = T[u].parent

    sib = ret_sibling(T, u)
    deg = ret_degree(T, u)
    node_type = 'internal node' if deg != 0 else 'leaf'
    if parent == -1:
        node_type = 'root'

    print(
        f'node {u}: parent = {parent}, sibling = {sib}, degree = {deg}, depth = {D[u]}, height = {H[u]}, {node_type}')


for node in range(n):
    print_for_a_node(node)
