# https://atcoder.jp/contests/abc023/tasks/abc023_d
# tの二分探索で行けそう


# 点数の二分探索じゃない？
# →風船を割り切ることができる中の最小のpが答え
# 風船を割り切る判定はどうする？
# 各風船に対してhiがpになるまでに必要な秒数tiがわかる
# 各風船はその秒数以内に割られないと行けない。そのためソートしたときにti<iになってしまう風船は時間内に割ることができない。

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


N = read_a_int()
H, S = read_col(N)


def is_ok(p):
    # 得点p以下となるように風船を割り切れか？
    # 切り捨てでいいかな ∵ その秒数がp以下となる整数値としての限界
    T = [(p - h) // s for h, s in zip(H, S)]
    T.sort()
    for i in range(N):
        if T[i] < i:
            return False
    return True


def meguru_bisect(ng, ok):
    '''
    define is_okと
    初期値のng,okを受け取り,is_okを満たす最小(最大)のokを返す
    ng ok は  とり得る最小の値-1 とり得る最大の値+1
    最大最小が逆の場合はよしなにひっくり返す
    '''
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if is_ok(mid):
            ok = mid
        else:
            ng = mid
    return ok


print(meguru_bisect(-1, 2 * 10**15))
