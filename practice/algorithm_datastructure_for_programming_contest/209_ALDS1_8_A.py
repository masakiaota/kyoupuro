# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/8/ALDS1_8_A
# 次への伏線みたいなもの
# ここらへんはもうひたすら実装するだけ
import sys
sys.setrecursionlimit(2**20)  # 再帰回数上限の向上 かなり多くしないとREになる


class Node:
    def __init__(self,
                 parent=None,
                 left=None,
                 right=None):
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


def insert(T, ROOT, z):
    '''
    zは接点番号(そしてその接点番号が内容でもある)
    この接点番号を二分探索木の条件に従うように挿入する。
    '''
    # もしTが空だった場合(最初の一回しか発動されない)
    if ROOT == z:
        T[z] = Node()
    # もしTにいくつか入っていた場合
    else:
        T[z] = Node()  # 初期化
        x = ROOT
        while x is not None:  # 子がNoneになったらbreakする
            next_parent = x  # 次のループではxの親になる。whileから抜けたときに使用する
            if z < x:  # もし新たなノードが現在のノードよりも小さければ左に移動する
                x = T[x].left
            else:
                x = T[x].right
        T[z].parent = next_parent
        # 親の右と左どっちにつける？
        if z < next_parent:
            T[next_parent].left = z
        else:
            T[next_parent].right = z


def print_result(T):
    pre_ls, in_ls = [], []
    in_parse(T, ROOT, in_ls)
    pre_parse(T, ROOT, pre_ls)
    print('', *in_ls)
    print('', *pre_ls)

    # データの読み込み
N = int(input())
T = {}
for i in range(N):
    tmp = input()
    if tmp.startswith('print'):  # なぜかtmp == 'print'ではerror
        print_result(T)
    elif tmp.startswith('insert'):
        command, z = tmp.split()[0], tmp.split()[1]
        z = int(z)
        if i == 0:
            ROOT = z
        insert(T, ROOT, z)
