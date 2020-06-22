import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_ints(): return list(map(int, read().split()))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


N, K = read_ints()
AB = read_tuple(N)
AB.sort()

# めちゃくちゃセグ木と二部探したくなるけど線形探索で十分

now = 0  # i番目の要素をすべて考慮したとき何番目の数になるか
# 初めてK以上になったときの数が答え
for a, b in AB:
    now += b
    if now >= K:
        print(a)
        exit()
