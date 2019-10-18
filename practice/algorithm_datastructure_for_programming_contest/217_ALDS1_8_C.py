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


def find(T, ROOT, k):
    '''
    Tからkを見つけ出す。見つからなかったらNoneを返す
    ひたすらkの大きさによって左右のパスをたどっていくだけ
    (いま辞書でノードを管理しているので、`k in T.keys()`で済むことではあるんだけど(オーダーは大きくなる)))
    '''
    x = ROOT  # 訪問中のノード
    while x is not None and x != k:
        if k < x:
            x = T[x].left
        else:
            x = T[x].right
    return x


def delete_node(T, z):
    # case1 指定したzが子を持たない場合
    parent = T[z].parent
    if (T[z].left is None) and (T[z].right is None):
        # そのノードを削除する
        if T[parent].left == z:
            T[parent].left = None
        elif T[parent].right == z:
            T[parent].right = None
        else:
            raise ValueError('something wrong in case 1')
        del T[z]
    # case2 指定したzが一つの子を保つ場合
    elif T[z].right is None:
        # 左だけある場合、左の子ノードを親につなぐ
        # 親の左右のどちらにつなぐかはzのつながり方に依存する
        z_is_left = True if T[parent].left == z else False
        child = T[z].left
        if z_is_left:
            T[parent].left = child
        else:
            T[parent].right = child
        T[child].parent = parent
        del T[z]
    elif T[z].left is None:
        # ROOTが片方の部分木しかない場合でROOTを削除しようとするコーナーケースに実は引っかかるはず(実は嘘解法)
        z_is_left = True if T[parent].left == z else False
        child = T[z].right
        if z_is_left:
            T[parent].left = child
        else:
            T[parent].right = child
        T[child].parent = parent
        del T[z]
    # case3 zが2つの子を保つ場合
    elif (T[z].left is not None) and (T[z].right is not None):
        # これに関しては本での解説に少し補足する
        left, right = T[z].left, T[z].right
        node_next_more = get_min_in_descendants(
            T, T[z].right)  # 指定したノードの子孫の中で一番小さいノードを返す #ここが問題だ node_next_moreがrightと一致するときにparentとかの代入がいろいろおかしくなる
        next_more_right = T[node_next_more].right
        next_more_parent = T[node_next_more].parent
        # zがROOTだったら
        if parent is None:
            if node_next_more == right:
                # node_next_more==rightとなるような場合
                T[right].parent = None
                T[right].left = left
                ROOT = right
                del T[z]
                return
            else:
                # node_next_moreがもっと下流にある場合
                # zの子と親との接続
                T[node_next_more].left = left
                T[node_next_more].right = right
                T[node_next_more].parent = None
                T[left].parent = node_next_more  # zの左側の接続
                T[right].parent = node_next_more  # zの右側の接続
                # node_next_moreの親 → node_next_more右子の接続 (node_next_moreの親からみると左に接続される)
                if next_more_right is not None:
                    T[next_more_right].parent = next_more_parent
                ROOT = node_next_more
                del T[z]
                return
        else:  # zが非ROOT
            # zが親からみてどっち側についているか
            z_is_left = True if T[parent].left == z else False
            if node_next_more == right:
                if z_is_left:
                    T[parent].left = right
                else:
                    T[parent].right = right
                T[right].parent = parent
                T[right].left = left
                del T[z]
                return
            else:  # 以下が多分もっとも一般的な場合
                # zの親->next_moreとの接続
                if z_is_left:
                    T[parent].left = node_next_more
                else:
                    T[parent].right = node_next_more
                # next_more -> zの子との接続
                T[node_next_more].left = T[z].left
                T[node_next_more].right = T[z].right
                T[node_next_more].parent = parent
                T[left].parent = node_next_more  # zの左側の接続
                T[right].parent = node_next_more  # zの左側の接続
                # node_next_moreの親 → node_next_more右子の接続 (node_next_moreの親からみると左に接続される)
                T[next_more_parent].left = next_more_right
                if next_more_right is not None:
                    T[next_more_right].parent = next_more_parent

                del T[z]
                return


def get_min_in_descendants(T, z):
    x = z
    while T[x].left is not None:
        x = T[x].left
    return x


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
        z = int(tmp.split()[1])
        if i == 0:
            ROOT = z
        insert(T, ROOT, z)
    elif tmp.startswith('find'):
        z = int(tmp.split()[1])
        if find(T, ROOT, z) is None:
            print('no')
        else:
            print('yes')
    elif tmp.startswith('delete'):
        z = int(tmp.split()[1])
        delete_node(T, z)
