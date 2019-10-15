# https://onlinejudge.u-aizu.ac.jp/courses/lesson/1/ALDS1/7/ALDS1_7_D
# 難しい 解き方は図を参照
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


# 答えの列を作るときに必要
def post_parse(T, u, post_ls):
    if u == None:
        return
    post_parse(T, T[u].left, post_ls)
    post_parse(T, T[u].right, post_ls)
    post_ls.append(u)


# データの読み込み
N = int(input())
pre_ls = list(map(int, input().split()))
in_ls = list(map(int, input().split()))
root_pos = 0
ROOT = pre_ls[root_pos]
T = {key+1: Node() for key in range(N)}

# 再帰関数で復元する


def rec(l: int, r: int, root_pos: int):
    '''
    l,rはin_lsに対して範囲を指定するidx
    l:rであり、半開区間
    root_posはpre_lsにおけるrootのidx。本では木の左下から復元するのを前提に実装していたが、
    ここではどういう順番で復元しても大丈夫なように実装する。
    '''
    # 即時終了条件
    if r-l <= 1:  # 指定区間が1ノード以下になったらおかしいので終了
        return
    root = pre_ls[root_pos]
    mid = in_ls.index(root)  # rootを持つ要素のidxを返す

    # 木の復元
    # 左部分木がある場合
    if mid != l:
        root_pos_left = root_pos+1  # preorderにおいては左部分木のroot_posは必ず現在の位置の隣になる
        T[pre_ls[root_pos]].left = pre_ls[root_pos_left]
        T[pre_ls[root_pos_left]].parent = pre_ls[root_pos]
        # 次の探索へ
        rec(l, mid, root_pos_left)
    # 右部分木がある場合
    if mid != r-1:  # rは一つ先を指定していることに注意すると-1に気付ける
        root_pos_right = root_pos+1+mid-l  # 右部分木のrootは、左部分木がおわった直後にある
        T[pre_ls[root_pos]].right = pre_ls[root_pos_right]
        T[pre_ls[root_pos_right]].parent = pre_ls[root_pos]
        rec(mid+1, r, root_pos_right)


# 木の復元
rec(0, N, root_pos)

# # print(T)
# for i in range(N):
#     print(T[i+1].parent, T[i+1].left, T[i+1].right)
# post orderの作成
post_ls = []
post_parse(T, ROOT, post_ls)

print(*post_ls)
