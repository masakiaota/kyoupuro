# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/7/ALDS1_7_A
# 図参照
# 配列を3つ用意してもよいがここではクラスを定義する。
# 本ではNILを-1としていたがここではNoneとして実装する（どっちのほうがいいんだろう）

import sys
sys.setrecursionlimit(2**20)  # 再帰回数上限の向上 かなり多くしないとREになる


class Node:
    def __init__(self,
                 parent,
                 left,
                 right):
        self.parent = parent
        self.left = left  # 一番左の**子**ノードということに注意
        self.right = right


def get_depth(T: dict, u: int):
    '''
    Tは木の構造と見立てたdict
    uはTのノード
    '''
    d = 0
    while T[u].parent is not None:
        u = T[u].parent
        d += 1  # 親をたどりながらdを増やしていく。


D = {}  # 深さを保存しておく用のdict


def get_all_depth(T: dict, u: int, p: int):
    '''
    一度にすべてのノードの深さを取得する関数(再帰的に処理を行う)。
    一気に全体の深さを求めるならばこちらの方がオーダーが小さい。
    pは現在扱う深さ
    '''
    D[u] = p
    # ココらへんの動きは図を参照、ふさごとに深さ優先探索している感じ
    if T[u].right is not None:
        # 右の兄弟ノード(深さは同じ)から入れていく
        get_all_depth(T, T[u].right, p)
    if T[u].left is not None:
        # 兄弟を入れ終わったら次は下のふさへ
        get_all_depth(T, T[u].left, p+1)


def ret_children(T, u):
    chilren = []
    c = T[u].left
    while c is not None:
        chilren.append(c)
        c = T[c].right
    return chilren

# 答え出力用


def print_for_a_node(T: dict, node: int, D: dict):
    node_type = 'internal node' if T[node].left is not None else 'leaf'
    if T[node].parent is None:
        parent = -1
        node_type = 'root'
    else:
        parent = T[node].parent
    children = ret_children(T, node)

    print(
        f'node {node}: parent = {parent}, depth = {D[node]}, {node_type}, {children}')


# この問題は入力をNoneに適する形に整形するのも少し難しい
n = int(input())

T = {k: Node(None, None, None) for k in range(n)}  # pythonぽくないけど初期化
for _ in range(n):
    tmp = list(map(int, input().split()))
    if tmp[1] == 0:  # もし子ノードがなければ次へ
        continue
    T[tmp[0]].left = tmp[2]  # 一番左の子ノードは即座にわかる
    T[tmp[2]].parent = tmp[0]  # 子の親ノードも即座に与えられる
    prev_sibling = tmp[2]  # 前のノード番号を覚えておく用
    for sib in tmp[3:]:
        T[prev_sibling].right = sib
        T[sib].parent = tmp[0]  # 子から親を与えている
        prev_sibling = sib

for node in range(n):
    if T[node].parent is None:
        get_all_depth(T, node, 0)  # rootを入れなければいけないことに注意
        # Dに深さが入った
        break

for node in range(n):
    print_for_a_node(T, node, D)
