import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque


class Top3:
    def __init__(self, ls):
        self.ls = ls

    def append(self, x):
        # xを突っ込む 3つのままに保つ
        # ついでに取れなくなったもののスコアを吐き出す
        self.ls.append(x)
        mi = min(self.ls)
        self.ls.remove(mi)
        return mi

    @property
    def sum(self):
        return sum(self.ls)


'''
dp[i][j]...(0,0)からスタートして(i,j)に到達する経路のなかでの最大の価値
dp[i][j]=max(dp[i][j-1],dp[i-1][j]) + v[i][j]

同じ行のアイテムは3つしか拾えないのはどうしようか...
各行でアイテムの価値をsortedlistに突っ込んでおくのは？ top3つ持ってればいいからそんなデータ構造いらない

dpだけじゃなくて行方向にいままでどんな要素を選んだか記録する必要あり
ritmとする。
横方向の移動があったときだけ中身を継承する(これはスコア算出に使う)
'''

R, C, K = ints()
RCV = read_tuple(K)
# values = defaultdict(lambda: 0)
values = [[0] * C for _ in ra(R)]
for r, c, v in RCV:
    values[r - 1][c - 1] = v

dp = [[-1] * (C + 5) for _ in ra(R + 5)]
dp[0][0] = values[0][0]
for i in ra(1, R):
    dp[i][0] = values[i][0] + dp[i - 1][0]

top3 = Top3(ls=[values[0][0], 0, 0])
for j in ra(1, C):
    dp[0][j] = dp[0][j - 1] + values[0][j] - \
        top3.append(values[0][j])

for i in ra(1, R):
    top3 = Top3(ls=[values[i][0], 0, 0])  # 行のitemをもつ
    for j in ra(1, C):
        top = dp[i - 1][j] + values[i][j]
        left = dp[i][j - 1] + values[i][j] - \
            top3.append(values[i][j])  # top3を考慮した横のスコア
        # top3.append(values[i][j])
        # left = top3.sum + dp[i][j - 1]

        # もしtopが大きければtop3は採用しない
        if top >= left:
            dp[i][j] = top
            top3 = Top3(ls=[values[i][j], 0, 0])
        else:
            dp[i][j] = left
            # appendしたtop3を次も継続して使う

print(dp[R - 1][C - 1])
# print(*values, sep='\n')
print(*dp, sep='\n')

# やってることはdp[i][j][k]のdp[i][j][3]だけ更新しているのと同じだと思うんだけどなんでかだめなんだよなぁ

# わかったーーー 上から来たときに、top3よりは小さいけど一つだけ選ぶ場合よりは大きいみたいな場合が考えられてない
# この実装だと行で1つだけ選んだ場合より大きい時はそのまま捨ててるけど、拾ったほうがあとあと大きくなる可能性があるね
