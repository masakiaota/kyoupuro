# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/7/ALDS1_7_C

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


def pre_parse(T: dict, u: int, pre_ls: list):
    # 深さ優先探索特有の即時終了条件
    if u == None:
        return
    pre_ls.append(u)
    pre_parse(T, T[u].left, pre_ls)  # より左の方から深さ優先探索
    pre_parse(T, T[u].right, pre_ls)


def in_parse(T, u, in_ls):
    if u == None:
        return
    # 左→中→右っていうちょっと気持ち悪い順番
    in_parse(T, T[u].left, in_ls)
    in_ls.append(u)
    in_parse(T, T[u].right, in_ls)


def post_parse(T, u, post_ls):
    if u == None:
        return
    post_parse(T, T[u].left, post_ls)
    post_parse(T, T[u].right, post_ls)
    post_ls.append(u)


# データの読み込み
n = int(input())
T = {key: Node(None, None, None) for key in range(n)}
for _ in range(n):
    tmp = [x if x != -1 else None for x in map(int, input().split())]
    T[tmp[0]].left = tmp[1]
    T[tmp[0]].right = tmp[2]
    if tmp[1] is not None:
        T[tmp[1]].parent = tmp[0]
    if tmp[2] is not None:
        T[tmp[2]].parent = tmp[0]

# parentがないのが特徴のROOTを探す
for id, node in T.items():
    if node.parent is None:
        ROOT = id

# リストに答えを格納していく
pre_ls, in_ls, post_ls = [], [], []
pre_parse(T, ROOT, pre_ls)
in_parse(T, ROOT, in_ls)
post_parse(T, ROOT, post_ls)

print('Preorder')
print('', *pre_ls)
print('Inorder')
print('', *in_ls)
print('Postorder')
print('', *post_ls)
