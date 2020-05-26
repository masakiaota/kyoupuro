import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split('-')))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


class Imos1d:
    def __init__(self, N: int):
        '''
        [0]*N の配列に対して、区間の加算を管理する。
        '''
        self.ls = [0] * (N + 1)  # 配列外参照を防ぐため多くとっておく
        self.N = N

    def add(self, l, r, x):
        '''
        [l,r)の区間にxを足し込む O(1)
        '''
        self.ls[l] += x
        self.ls[min(r, self.N)] -= x  # 配列外参照は余分に作ったところにまとめておく(どうせ使わない)

    def get_result(self):
        '''
        O(N) かけて、区間の加算結果を取得する
        '''
        from itertools import accumulate
        return list(accumulate(self.ls[:-1]))


def to_hour_minute(x):
    h = x // 60
    m = x % 60
    return str(100 * h + m).zfill(4)


def to_minutes(x):
    h = x // 100
    m = x - h * 100
    return 60 * h + m


N = read_a_int()
# (開始時刻,終了時刻)をsetに格納してソート
data = set()
for _ in ra(N):
    S, E = read_ints()
    # 四捨五入
    S = S // 5 * 5
    E = ((E - 1) // 5 + 1) * 5
    data.add((to_minutes(S), to_minutes(E)))

# imos法で雨が降った区間を塗る
imos = Imos1d(24 * 60 + 1)
for s, t in data:
    imos.add(s, t, 1)
res = imos.get_result()

pre_is_rain = False
ans_tmp = []
for i, r in enu(res):
    if (pre_is_rain == False and r) or (pre_is_rain and r == 0):
        # 雨と晴れが切り替わる点を記録
        ans_tmp.append(i)
    pre_is_rain = r > 0

ans = []
for a, b in zip(ans_tmp[::2], ans_tmp[1::2]):
    ans.append(to_hour_minute(a) + '-' + to_hour_minute(b))

print(*ans, sep='\n')
