import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


from collections import Counter


class FastFactorization:
    def __init__(self, N: int):
        '''構築O(NloglogN)、クエリO(logN)'''
        self.N = N
        self.min_prime = self._make_minimum_prime()

    def _make_minimum_prime(self):
        # xの最小の素因数表を作成
        min_prime = [x for x in range(self.N + 1)]
        # min_prime[0] = 0  # 0と1は素数ではない
        # min_prime[1] = 1
        for i in range(2, int(self.N ** 0.5) + 1):
            if min_prime[i] == i:  # 素数だったら更新
                for j in range(2 * i, self.N + 1, i):  # iの倍数は素数でない
                    if min_prime[j] == j:
                        min_prime[j] = i
        return min_prime

    def query(self, x: int):
        # -> Counter[p,n] (素数,冪数) を格納
        # 最小素数配列min_primeを使ってO(log N)で因数分解
        if x == 1:
            return Counter()  # 1は素数ではない

        # 素因数分解
        arr = []
        tmp = x
        while tmp != 1:
            p = self.min_prime[tmp]
            tmp //= p
            arr.append(p)
        return Counter(arr)


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from math import gcd


N = a_int()
A = ints()

fact = FastFactorization(max(A))

# setかはすぐわかる
# setでなければ not coprime
# pairは互いに素かをみればいいのか
# つまり因数分解して足してったときにすべての素数のべき数が1以下であれば良い

g_set = 0
cnt = defaultdict(lambda: 0)
flg = 1  # pairwiseであるフラグ
for a in A:
    g_set = gcd(g_set, a)
    if flg:
        for p, n in fact.query(a).items():
            if cnt[p] != 0:
                flg = 0
            cnt[p] += n


if g_set > 1:
    print('not coprime')
elif flg:
    print('pairwise coprime')
else:
    print('setwise coprime')
